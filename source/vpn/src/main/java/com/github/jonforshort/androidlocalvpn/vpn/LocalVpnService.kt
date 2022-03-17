//
// This is free and unencumbered software released into the public domain.
//
// Anyone is free to copy, modify, publish, use, compile, sell, or
// distribute this software, either in source code form or as a compiled
// binary, for any purpose, commercial or non-commercial, and by any
// means.
//
// In jurisdictions that recognize copyright laws, the author or authors
// of this software dedicate any and all copyright interest in the
// software to the public domain. We make this dedication for the benefit
// of the public at large and to the detriment of our heirs and
// successors. We intend this dedication to be an overt act of
// relinquishment in perpetuity of all present and future rights to this
// software under copyright law.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
// OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
// OTHER DEALINGS IN THE SOFTWARE.
//
// For more information, please refer to <https://unlicense.org>
//
package com.github.jonforshort.androidlocalvpn.vpn

import android.app.ActivityManager
import android.content.Context
import android.content.Intent
import android.net.VpnService
import android.os.ParcelFileDescriptor
import timber.log.Timber.d
import timber.log.Timber.e
import java.io.IOException
import java.net.NetworkInterface

fun startVpn(context: Context) {
    val intent = Intent(context, LocalVpnService::class.java).apply {
        action = "START_VPN"
    }
    context.startService(intent)
}

fun stopVpn(context: Context) {
    val intent = Intent(context, LocalVpnService::class.java).apply {
        action = "STOP_VPN"
    }
    context.startService(intent)
}

fun isVpnRunning(context: Context) = isVpnTunnelUp() && isVpnServiceRunning(context)

@Suppress("DEPRECATION")
private fun isVpnServiceRunning(context: Context) =
    (context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager)
        .getRunningServices(Integer.MAX_VALUE)
        .any { it.service.className == LocalVpnService::class.java.name }

private fun isVpnTunnelUp(): Boolean {
    val networkInterfaces = NetworkInterface.getNetworkInterfaces()
    for (networkInterface in networkInterfaces) {
        if (networkInterface.displayName == "tun0" && networkInterface.isUp) {
            return true
        }
    }
    return false
}

class LocalVpnService : VpnService() {

    private lateinit var vpnInterface: ParcelFileDescriptor

    companion object {
        private const val VPN_ADDRESS = "10.0.0.2"
        private const val VPN_ROUTE = "0.0.0.0"

        init {
            System.loadLibrary("vpn")
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == "STOP_VPN") {
            stopVpn()
        }
        return START_STICKY
    }

    private fun stopVpn() {
        try {
            vpnInterface.close()
        } catch (e: IOException) {
            e(e, "unable to close parcel file descriptor")
        }
        onStopVpn()
        stopForeground(true)
        stopSelf()
    }

    override fun onCreate() {
        super.onCreate()
        d("onCreate called")
        setUpVpnInterface()
        onCreateNative(this)
        onStartVpn(vpnInterface.detachFd())
    }

    private fun setUpVpnInterface() {
        d("setting up vpn interface")

        val vpnServiceBuilder = super.Builder()
        vpnServiceBuilder.addAddress(VPN_ADDRESS, 32)
        vpnServiceBuilder.addRoute(VPN_ROUTE, 0)

        vpnInterface = vpnServiceBuilder
            .setBlocking(false)
            .setSession("LocalVpnService")
            .establish()!!
    }

    override fun onDestroy() {
        super.onDestroy()
        d("onDestroy called")
        onDestroyNative()
    }

    private external fun onCreateNative(vpnService: VpnService)

    private external fun onDestroyNative()

    private external fun onStartVpn(fileDescriptor: Int)

    private external fun onStopVpn()
}
