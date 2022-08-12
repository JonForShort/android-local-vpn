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

import android.net.VpnService
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts.StartActivityForResult
import androidx.compose.foundation.layout.*
import androidx.compose.material.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Send
import androidx.compose.material.icons.filled.Settings
import androidx.compose.runtime.*
import androidx.compose.runtime.livedata.observeAsState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.lifecycle.MutableLiveData
import com.github.jonforshort.androidlocalvpn.ui.theme.AndroidLocalVpnTheme
import com.github.jonforshort.androidlocalvpn.vpn.isVpnRunning
import com.github.jonforshort.androidlocalvpn.vpn.startVpn
import com.github.jonforshort.androidlocalvpn.vpn.stopVpn
import com.google.accompanist.pager.ExperimentalPagerApi
import com.google.accompanist.pager.HorizontalPager
import com.google.accompanist.pager.pagerTabIndicatorOffset
import com.google.accompanist.pager.rememberPagerState
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import org.jsoup.Connection
import org.jsoup.Jsoup
import timber.log.Timber.d
import timber.log.Timber.e
import java.io.IOException

class MainActivity : ComponentActivity() {

    private val vpnState = MutableLiveData(false)

    private val vpnServicePreparedLauncher =
        registerForActivityResult(StartActivityForResult()) { result ->
            if (result.resultCode == RESULT_OK) {
                startVpn(this)
                vpnState.postValue(true)
            }
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            AndroidLocalVpnTheme {
                Surface(color = MaterialTheme.colors.background) {
                    MainView(
                        tabData = listOf(
                            "TEST" to Icons.Filled.Send,
                            "SETTINGS" to Icons.Filled.Settings,
                        ),
                        onTabDisplayed = { index ->
                            when (index) {
                                0 -> {
                                    TestTab()
                                }
                                1 -> {
                                    SettingsTab(
                                        vpnState.observeAsState(),
                                        ::onVpnStateChanged
                                    )
                                }
                            }
                        }
                    )
                }
            }
        }

        vpnState.postValue(isVpnRunning(this))
    }

    private fun onVpnStateChanged(vpnEnabled: Boolean) {
        if (vpnEnabled) {
            val vpnIntent = VpnService.prepare(this)
            if (vpnIntent == null) {
                startVpn(this)
                vpnState.postValue(true)
            } else {
                vpnServicePreparedLauncher.launch(vpnIntent)
            }
        } else {
            stopVpn(this)
            vpnState.postValue(false)
        }
    }
}

@Composable
private fun TestTab() {

    Column {
        TestButton(
            text = "Google (HTTP)",
            url = "http://google.com/"
        )

        TestButton(
            text = "Google (HTTPS)",
            url = "https://google.com/"
        )

        TestButton(
            text = "HttpBin (HTTP)",
            url = "http://httpbin.org"
        )

        TestButton(
            text = "HttpBin (HTTPS)",
            url = "https://httpbin.org"
        )

        TestButton(
            text = "Kernel (HTTP)",
            url = "http://mirrors.edge.kernel.org/pub/site/README"
        )

        TestButton(
            text = "Kernel (HTTPS)",
            url = "https://mirrors.edge.kernel.org/pub/site/README"
        )
    }
}

@Composable
private fun TestButton(text: String, url: String) {
    val coroutineScope = rememberCoroutineScope()

    fun performJsoupRequest(onRequestStarted: () -> Unit, onRequestFinished: (Boolean) -> Unit) {
        coroutineScope.launch(Dispatchers.IO) {
            onRequestStarted()
            val conn = Jsoup
                .connect(url)
                .followRedirects(false)
                .method(Connection.Method.GET)
            try {
                val resp = conn.execute()
                val html = resp.body()
                d("dumping html, count=${html.length}")
                d(html)
                d("done dumping html")
                onRequestFinished(true)
            } catch (e: IOException) {
                e(e)
                onRequestFinished(false)
            }
        }
    }

    val buttonColor = remember { mutableStateOf(Color.Magenta) }

    Button(
        modifier = Modifier.fillMaxWidth(),
        colors = ButtonDefaults.buttonColors(backgroundColor = buttonColor.value),
        onClick = {
            performJsoupRequest(
                onRequestStarted = {
                    buttonColor.value = Color.LightGray
                },
                onRequestFinished = { isSuccessful ->
                    if (isSuccessful) {
                        buttonColor.value = Color.Green
                    } else {
                        buttonColor.value = Color.Red
                    }
                })
        }
    ) {
        Text(text)
    }
}

@Composable
private fun SettingsTab(
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

@OptIn(ExperimentalPagerApi::class)
@Composable
private fun MainView(
    tabData: List<Pair<String, ImageVector>> = emptyList(),
    onTabDisplayed: @Composable (index: Int) -> Unit = {}
) {
    val pagerState = rememberPagerState()
    val tabIndex = pagerState.currentPage
    val coroutineScope = rememberCoroutineScope()
    Column {
        TabRow(
            selectedTabIndex = tabIndex,
            indicator = { tabPositions ->
                TabRowDefaults.Indicator(
                    Modifier.pagerTabIndicatorOffset(pagerState, tabPositions)
                )
            }
        ) {
            tabData.forEachIndexed { index, pair ->
                Tab(selected = tabIndex == index, onClick = {
                    coroutineScope.launch {
                        pagerState.animateScrollToPage(index)
                    }
                }, text = {
                    Text(text = pair.first)
                }, icon = {
                    Icon(imageVector = pair.second, contentDescription = null)
                })
            }
        }
        HorizontalPager(
            state = pagerState,
            modifier = Modifier.weight(1f),
            count = tabData.size
        ) { index ->
            Column(
                modifier = Modifier.fillMaxSize(),
                verticalArrangement = Arrangement.Top,
                horizontalAlignment = Alignment.Start
            ) {
                onTabDisplayed(index)
            }
        }
    }
}

@Preview(showBackground = true)
@Composable
private fun DefaultPreview() {
    AndroidLocalVpnTheme {
        MainView()
    }
}