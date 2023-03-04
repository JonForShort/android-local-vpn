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
package com.github.jonforshort.androidlocalvpn.ui.main

import android.os.Bundle
import androidx.activity.compose.setContent
import androidx.compose.runtime.livedata.observeAsState
import androidx.lifecycle.MutableLiveData
import com.github.jonforshort.androidlocalvpn.vpn.LocalVpnActivity
import com.github.jonforshort.androidlocalvpn.vpn.LocalVpnConfiguration
import com.github.jonforshort.androidlocalvpn.vpn.PackageName

class MainActivity : LocalVpnActivity() {

    private val vpnState = MutableLiveData(false)

    private lateinit var mainViewModel: MainViewModel

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        mainViewModel = MainViewModel(application)

        setContent {
            MainScreen(
                mainViewModel = mainViewModel,
                isVpnEnabled = vpnState.observeAsState(),
                onVpnEnabledChanged = ::onVpnStateChanged
            )
        }

        vpnState.postValue(isVpnRunning())
    }

    override fun onResume() {
        super.onResume()
        mainViewModel.refresh()
    }

    private fun onVpnStateChanged(vpnEnabled: Boolean) =
        if (vpnEnabled) {
            val configuration = buildConfiguration()
            startVpn(configuration)
        } else {
            stopVpn()
        }

    private fun buildConfiguration(): LocalVpnConfiguration {
        val allowedApps = mutableListOf<PackageName>()
        val disallowedApps = mutableListOf<PackageName>()

        mainViewModel.applicationSettings.value.forEach {
            when (it.policy) {
                VpnPolicy.ALLOW -> allowedApps.add(PackageName(it.packageName))
                VpnPolicy.DISALLOW -> disallowedApps.add(PackageName(it.packageName))
                else -> {}
            }
        }

        return LocalVpnConfiguration(allowedApps, disallowedApps)
    }

    override fun onVpnStarted() = vpnState.postValue(true)

    override fun onVpnStopped() = vpnState.postValue(false)
}

