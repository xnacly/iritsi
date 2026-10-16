package dev.dioxus.main

import android.Manifest
import android.app.AlarmManager
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import android.view.WindowInsetsController
import org.json.JSONArray
import org.json.JSONObject

typealias BuildConfig = me.xnacly.iritsi.BuildConfig

class MainActivity : WryActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val light = WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS or
            WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS
        window.decorView.windowInsetsController?.setSystemBarsAppearance(light, light)

        Alerts.createChannel(this)
        if (checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 1)
        }
    }

    fun scheduleAlerts(json: String) {
        Alerts.schedule(this, JSONArray(json))
    }

    fun cancelAlerts() {
        Alerts.cancel(this)
    }
}

class AlertReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        Alerts.fire(context, intent.getIntExtra(Alerts.INDEX, -1))
    }
}

object Alerts {
    const val INDEX = "index"
    private const val CHANNEL = "trips"
    private const val NOTIFICATION = 1
    private const val PREFS = "watch"
    private const val ALERTS = "alerts"
    private const val REQUEST_PROMOTED_ONGOING = "android.requestPromotedOngoing"

    fun createChannel(context: Context) {
        manager(context).createNotificationChannel(
            NotificationChannel(CHANNEL, "Trips", NotificationManager.IMPORTANCE_HIGH)
        )
    }

    fun schedule(context: Context, alerts: JSONArray) {
        cancel(context)
        prefs(context).edit().putString(ALERTS, alerts.toString()).apply()

        val alarms = context.getSystemService(AlarmManager::class.java)
        val now = System.currentTimeMillis()
        var current = -1
        for (i in 0 until alerts.length()) {
            val at = alerts.getJSONObject(i).getLong("at") * 1000
            if (at <= now) {
                current = i
                continue
            }
            val pending = pending(context, i)
            if (alarms.canScheduleExactAlarms()) {
                alarms.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, at, pending)
            } else {
                alarms.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, at, pending)
            }
        }
        if (current >= 0) {
            fire(context, current)
        }
    }

    fun cancel(context: Context) {
        val alarms = context.getSystemService(AlarmManager::class.java)
        val count = stored(context)?.length() ?: 0
        for (i in 0 until count) {
            alarms.cancel(pending(context, i))
        }
        prefs(context).edit().remove(ALERTS).apply()
        manager(context).cancel(NOTIFICATION)
    }

    fun fire(context: Context, index: Int) {
        val alerts = stored(context) ?: return
        if (index !in 0 until alerts.length()) return
        val alert = alerts.getJSONObject(index)
        if (alert.optBoolean("done")) {
            prefs(context).edit().remove(ALERTS).apply()
            manager(context).cancel(NOTIFICATION)
            return
        }
        manager(context).notify(NOTIFICATION, build(context, alert))
    }

    private fun build(context: Context, alert: JSONObject): Notification {
        val open = PendingIntent.getActivity(
            context,
            0,
            Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
            PendingIntent.FLAG_IMMUTABLE,
        )
        val builder = Notification.Builder(context, CHANNEL)
            .setSmallIcon(icon(context))
            .setContentTitle(alert.getString("title"))
            .setContentText(alert.getString("text"))
            .setShortCriticalText(alert.getString("chip"))
            .setCategory(Notification.CATEGORY_NAVIGATION)
            .setContentIntent(open)
            .setOngoing(true)
            .setOnlyAlertOnce(!alert.getBoolean("buzz"))
            .setColor(alert.getInt("accent"))

        if (alert.has("countdown_to")) {
            builder.setWhen(alert.getLong("countdown_to") * 1000)
                .setShowWhen(true)
                .setUsesChronometer(true)
                .setChronometerCountDown(true)
        }

        val progress = alert.optJSONObject("progress")
        if (progress != null) {
            val style = Notification.ProgressStyle()
                .setStyledByProgress(true)
                .setProgress(progress.getInt("value"))
            val segments = progress.getJSONArray("segments")
            for (i in 0 until segments.length()) {
                val segment = segments.getJSONObject(i)
                style.addProgressSegment(
                    Notification.ProgressStyle.Segment(segment.getInt("length"))
                        .setColor(segment.getInt("color"))
                )
            }
            val points = progress.getJSONArray("points")
            for (i in 0 until points.length()) {
                style.addProgressPoint(
                    Notification.ProgressStyle.Point(points.getInt(i))
                        .setColor(progress.getInt("point_color"))
                )
            }
            builder.setStyle(style)
        }

        builder.extras.putBoolean(REQUEST_PROMOTED_ONGOING, true)
        return builder.build()
    }

    private fun pending(context: Context, index: Int): PendingIntent =
        PendingIntent.getBroadcast(
            context,
            index,
            Intent(context, AlertReceiver::class.java).putExtra(INDEX, index),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )

    private fun stored(context: Context): JSONArray? =
        prefs(context).getString(ALERTS, null)?.let { JSONArray(it) }

    private fun prefs(context: Context) =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    private fun manager(context: Context): NotificationManager =
        context.getSystemService(NotificationManager::class.java)

    private fun icon(context: Context): Int =
        context.resources.getIdentifier("ic_notification", "drawable", context.packageName)
            .takeIf { it != 0 } ?: context.applicationInfo.icon
}
