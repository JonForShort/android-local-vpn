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
package com.github.jonforshort.androidlocalvpn

import android.content.Intent
import android.net.VpnService
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Surface
import androidx.compose.material.Switch
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.runtime.livedata.observeAsState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.lifecycle.MutableLiveData
import com.github.jonforshort.androidlocalvpn.ui.theme.AndroidLocalVpnTheme
import com.github.jonforshort.androidlocalvpn.vpn.isVpnRunning
import com.github.jonforshort.androidlocalvpn.vpn.startVpn
import com.github.jonforshort.androidlocalvpn.vpn.stopVpn

class MainActivity : ComponentActivity() {

    private val vpnState = MutableLiveData(false)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            AndroidLocalVpnTheme {
                Surface(color = MaterialTheme.colors.background) {
                    VpnState(
                        vpnState.observeAsState(),
                        ::onVpnStateChanged
                    )
                }
            }
        }

        vpnState.postValue(isVpnRunning(this))
    }

    private fun onVpnStateChanged(vpnEnabled: Boolean) {
        if (vpnEnabled) {
            prepareVpn()
        } else {
            stopVpn(this)
            vpnState.postValue(false)
        }
    }

    private fun prepareVpn() {
        val vpnIntent = VpnService.prepare(this)
        if (vpnIntent != null) {
            startActivityForResult(vpnIntent, LOCAL_VPN_REQUEST_CODE)
        } else {
            onActivityResult(LOCAL_VPN_REQUEST_CODE, RESULT_OK, null)
        }
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == LOCAL_VPN_REQUEST_CODE && resultCode == RESULT_OK) {
            startVpn(this)
            vpnState.postValue(true)
        }
    }
}

private const val LOCAL_VPN_REQUEST_CODE = 1000

@Composable
private fun VpnState(
    isVpnEnabled: State<Boolean?> = mutableStateOf(false),
    onVpnEnabledChanged: (Boolean) -> Unit = {}
) {
    Row(
        modifier = Modifier
            .height(IntrinsicSize.Max)
            .padding(10.dp)
            .fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween
    )
    {
        Text(
            text = "Enable VPN",
        )
        Switch(
            checked = isVpnEnabled.value ?: false,
            onCheckedChange = { onVpnEnabledChanged(it) },
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun DefaultPreview() {
    AndroidLocalVpnTheme {
        VpnState()
    }
}