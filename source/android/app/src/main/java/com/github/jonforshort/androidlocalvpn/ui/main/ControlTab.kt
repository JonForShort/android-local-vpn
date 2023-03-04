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
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Switch
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import com.github.jonforshort.androidlocalvpn.ui.theme.AndroidLocalVpnTheme

@Composable
internal fun ControlTab(
    isVpnEnabled: Boolean,
    onVpnEnabledChanged: (Boolean) -> Unit,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier) {
        EnableVpnToggle(
            isVpnEnabled, onVpnEnabledChanged
        )
    }
}

@Composable
private fun EnableVpnToggle(
    isVpnEnabled: Boolean,
    onVpnEnabledChanged: (Boolean) -> Unit
) {
    Row(
        modifier = Modifier
            .height(IntrinsicSize.Max)
            .fillMaxSize(),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween
    ) {
        Text(
            text = "Enable VPN",
            style = MaterialTheme.typography.h4
        )
        Switch(
            checked = isVpnEnabled,
            onCheckedChange = { onVpnEnabledChanged(it) },
        )
    }
}

@Preview
@Composable
fun ControlsTabPreview() {
    AndroidLocalVpnTheme {
        ControlTab(
            isVpnEnabled = false,
            onVpnEnabledChanged = {},
        )
    }
}