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
import android.os.Build.VERSION
import android.os.Build.VERSION_CODES
import android.os.ParcelFileDescriptor
import android.os.Parcelable
import com.github.jonforshort.androidlocalvpn.vpn.LocalVpnService.Companion.INTENT_ACTION_START_VPN
import com.github.jonforshort.androidlocalvpn.vpn.LocalVpnService.Companion.INTENT_ACTION_STOP_VPN
import com.github.jonforshort.androidlocalvpn.vpn.LocalVpnService.Companion.INTENT_EXTRA_CONFIGURATION
import timber.log.Timber.e
import java.io.IOException
import java.net.NetworkInterface

internal fun startVpn(context: Context, configuration: LocalVpnConfiguration) {
    val intent = Intent(context, LocalVpnService::class.java).apply {
        action = INTENT_ACTION_START_VPN
        putExtra(INTENT_EXTRA_CONFIGURATION, configuration)
    }
    context.startService(intent)
}

internal fun stopVpn(context: Context) {
    val intent = Intent(context, LocalVpnService::class.java).apply {
        action = INTENT_ACTION_STOP_VPN
    }
    context.startService(intent)
}

internal fun isVpnRunning(context: Context) = isVpnTunnelUp() && isVpnServiceRunning(context)

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

internal class LocalVpnService : VpnService() {

    private lateinit var vpnInterface: ParcelFileDescriptor

    companion object {
        private const val VPN_ADDRESS = "10.0.0.2"
        private const val VPN_ROUTE = "0.0.0.0"

        internal const val INTENT_ACTION_START_VPN = "LocalVpnServiceStartVpn"
        internal const val INTENT_ACTION_STOP_VPN = "LocalVpnServiceStopVpn"
        internal const val INTENT_EXTRA_CONFIGURATION = "LocalVpnServiceConfiguration"

        init {
            System.loadLibrary("vpn")
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            INTENT_ACTION_START_VPN -> {
                startVpn(configuration = intent.getParcelableExtraCompat(INTENT_EXTRA_CONFIGURATION))
            }

            INTENT_ACTION_STOP_VPN -> {
                stopVpn()
            }
        }
        return START_STICKY
    }

    private fun stopVpn() {
        onStopVpn()
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
        closeVpnInterface()
    }

    private fun closeVpnInterface() {
        try {
            vpnInterface.close()
        } catch (e: IOException) {
            e(e, "failed to close parcel file descriptor")
        }
    }

    private fun startVpn(configuration: LocalVpnConfiguration?) {
        setUpVpnInterface(configuration)
        onCreateNative(this)
        onStartVpn(vpnInterface.detachFd())
    }

    private fun setUpVpnInterface(configuration: LocalVpnConfiguration?) {
        val vpnServiceBuilder = super.Builder().apply {
            addAddress(VPN_ADDRESS, 32)
            addRoute(VPN_ROUTE, 0)
        }

        configuration?.allowedApps?.forEach {
            vpnServiceBuilder.addAllowedApplication(it.packageName)
        }

        configuration?.disallowedApps?.forEach {
            vpnServiceBuilder.addDisallowedApplication(it.packageName)
        }

        vpnInterface = vpnServiceBuilder
            .setBlocking(false)
            .setSession("LocalVpnService")
            .establish()!!
    }

    override fun onDestroy() {
        super.onDestroy()
        onDestroyNative()
    }

    private external fun onCreateNative(vpnService: VpnService)

    private external fun onDestroyNative()

    private external fun onStartVpn(fileDescriptor: Int)

    private external fun onStopVpn()
}

private inline fun <reified T : Parcelable> Intent.getParcelableExtraCompat(key: String) = when {
    VERSION.SDK_INT >= VERSION_CODES.TIRAMISU -> getParcelableExtra(key, T::class.java)
    else -> @Suppress("DEPRECATION") getParcelableExtra(key) as? T
}