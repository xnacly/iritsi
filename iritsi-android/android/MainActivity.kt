package dev.dioxus.main

import android.os.Bundle
import android.view.WindowInsetsController

typealias BuildConfig = me.xnacly.iritsi.BuildConfig

class MainActivity : WryActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val light = WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS or
            WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS
        window.decorView.windowInsetsController?.setSystemBarsAppearance(light, light)
    }
}
