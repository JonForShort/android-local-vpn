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

import androidx.compose.foundation.layout.*
import androidx.compose.material.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Send
import androidx.compose.material.icons.filled.Settings
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.tooling.preview.Preview
import com.github.jonforshort.androidlocalvpn.ui.theme.AndroidLocalVpnTheme
import com.google.accompanist.pager.ExperimentalPagerApi
import com.google.accompanist.pager.HorizontalPager
import com.google.accompanist.pager.pagerTabIndicatorOffset
import com.google.accompanist.pager.rememberPagerState
import kotlinx.coroutines.launch
import org.xbill.DNS.*

@Composable
internal fun MainScreen(
    isVpnEnabled: State<Boolean?> = mutableStateOf(false),
    onVpnEnabledChanged: (Boolean) -> Unit
) {
    val tabs = listOf(
        MainScreenTab(
            tabName = "SETTINGS",
            tabIcon = Icons.Filled.Settings,
            tab = {
                SettingsTab(
                    isVpnEnabled,
                    onVpnEnabledChanged
                )
            }
        ),
        MainScreenTab(
            tabName = "TEST",
            tabIcon = Icons.Filled.Send,
            tab = { TestTab() }
        ),
    )

    AndroidLocalVpnTheme {
        Surface(color = MaterialTheme.colors.background) {
            MainView(
                tabData = tabs.map { it.tabName to it.tabIcon },
                onTabDisplayed = { tabs[it].tab() }
            )
        }
    }
}

private data class MainScreenTab(
    val tabName: String,
    val tabIcon: ImageVector,
    val tab: @Composable () -> Unit
)

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