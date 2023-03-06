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

import android.app.Application
import android.content.Context
import android.content.pm.PackageManager
import androidx.lifecycle.AndroidViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

internal class MainViewModel(application: Application) : AndroidViewModel(application) {

    private val applicationSettingsStore =
        application.getSharedPreferences(
            SHARED_PREFERENCES_APPLICATION_SETTINGS_KEY,
            Context.MODE_PRIVATE
        )

    private val vpnPolicySettingsStore =
        application.getSharedPreferences(SHARED_PREFERENCES_VPN_POLICY_KEY, Context.MODE_PRIVATE)

    private val _applicationsSettingsState =
        MutableStateFlow(emptyList<ApplicationSettings>())

    val applicationSettings: StateFlow<List<ApplicationSettings>>
        get() = _applicationsSettingsState.asStateFlow()

    private val _vpnPolicy =
        MutableStateFlow(VpnPolicy.DEFAULT)

    val vpnPolicy: StateFlow<VpnPolicy>
        get() = _vpnPolicy.asStateFlow()

    fun refresh() {
        _applicationsSettingsState.value = getInstalledApplications()
        _vpnPolicy.value = vpnPolicySettingsStore.getString(SHARED_PREFERENCES_VPN_POLICY_KEY, null)
            ?.let { VpnPolicy.valueOf(it) }
            ?: VpnPolicy.DEFAULT
    }

    fun reset() {
        applicationSettingsStore.edit().clear().apply()
        vpnPolicySettingsStore.edit().clear().apply()

        refresh()
    }

    private fun getInstalledApplications() = mutableListOf<ApplicationSettings>().apply {
        val packageManager = getApplication<Application>()
            .applicationContext
            .packageManager

        val installedApplications = packageManager
            .getInstalledApplications(PackageManager.GET_META_DATA)

        installedApplications.forEach { installedApplication ->
            add(
                ApplicationSettings(
                    packageName = installedApplication.packageName,
                    appIcon = installedApplication.loadIcon(packageManager),
                    appName = installedApplication.loadLabel(packageManager).toString(),
                    policy = applicationSettingsStore.getString(
                        installedApplication.packageName,
                        VpnPolicy.DEFAULT.name
                    )!!.let {
                        VpnPolicy.valueOf(it)
                    }
                )
            )
        }
    }

    fun adjustApplicationSettings(
        vpnPolicy: VpnPolicy,
        applicationSettingsList: List<ApplicationSettings>
    ) {
        applicationSettingsStore.edit().apply {
            applicationSettingsList.forEach {
                putString(it.packageName, vpnPolicy.name)
            }
        }.apply()

        vpnPolicySettingsStore.edit()
            .putString(SHARED_PREFERENCES_VPN_POLICY_KEY, vpnPolicy.name)
            .apply()

        refresh()
    }

    companion object {
        private const val SHARED_PREFERENCES_VPN_POLICY_KEY = "VpnPolicy"
        private const val SHARED_PREFERENCES_APPLICATION_SETTINGS_KEY = "ApplicationSettings"
    }
}