# Iritsi

<p align="center">
  <img width="256" height="256" src="https://raw.githubusercontent.com/xnacly/iritsi/refs/heads/master/assets/icons/128x128.png"/>
</p>
 
A rust offline first bilbao public transportation android application.

## Features:

- Search and Favorite stations
- Trach travels on both the metro bilbao map and a live notification
- etc.

## Local setup

### Dependencies

> Install android studio first, with:
> 1. sdk
> 2. sdk cli tools
> 3. ndk side by side
> 4. cmake
>
> See [Dioxus Mobile App>Android](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile#android)

Then add rust targets:

```shell
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

### Path setup

Then define:

```shell
export ANDROID_HOME=
export ANDROID_SDK_ROOT=
export ANDROID_NDK_HOME=
```

> For me its:
> 
> ```shell
> export ANDROID_HOME=/home/teo/Android/Sdk
> export ANDROID_SDK_ROOT=/home/teo/Android/Sdk
> export ANDROID_NDK_HOME=/home/teo/Android/Sdk/ndk/30.0.16138531
> ```

Also add the platform tools and the emulator
to your path:

```shell
export PATH="$PATH:$ANDROID_HOME/emulator:$ANDROID_HOME/platform-tools:$ANDROID_HOME/cmdline-tools/latest/bin"
```

### Android emulator configuration

By default there is no sdk installed, so we need to first install the SDK, I
went with 37.0 here, which is Android 17:

```shell
android sdk install \
    platform-tools \
    emulator \
    platforms/android-37.0 \
    system-images/android-37.0/google_apis/x86_64
```

Then we create an emulator (technically an android virtual device \[AVD\]):

```shell
# omitting --profile results in it being named medium_phone
android emulator create
```

And then we can finally use it to actually start it:

```shell
android emulator start medium_phone
```
