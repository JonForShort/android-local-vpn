#!/bin/bash

NDK_VERSION=21.1.6352462
PACKAGE_NAME=com.github.jonforshort.androidlocalvpn
PORT=9999

adb root

adb forward tcp:${PORT} tcp:${PORT}

adb push ${ANDROID_SDK_ROOT}/ndk/${NDK_VERSION}/toolchains/llvm/prebuilt/linux-x86_64/lib64/clang/9.0.8/lib/linux/x86_64/lldb-server /data/local/tmp/lldb-server

adb shell chmod +x /data/local/tmp/lldb-server

adb shell ps -A | grep ${PACKAGE_NAME}

echo "running lldb server"

adb shell /data/local/tmp/lldb-server platform --listen "*:${PORT}" --server

echo "done"
