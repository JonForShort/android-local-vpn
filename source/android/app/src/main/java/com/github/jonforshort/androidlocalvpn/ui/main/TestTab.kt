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

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material.Button
import androidx.compose.material.ButtonDefaults
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import org.jsoup.Connection
import org.jsoup.Jsoup
import org.xbill.DNS.*
import timber.log.Timber
import java.io.IOException
import java.net.SocketTimeoutException

@Composable
internal fun TestTab() {

    Column {
        TestHtmlQuery(
            text = "Google (HTTP)",
            url = "http://google.com/"
        )

        TestHtmlQuery(
            text = "Google (HTTPS)",
            url = "https://google.com/"
        )

        TestHtmlQuery(
            text = "HttpBin (HTTP)",
            url = "http://httpbin.org"
        )

        TestHtmlQuery(
            text = "HttpBin (HTTPS)",
            url = "https://httpbin.org"
        )

        TestHtmlQuery(
            text = "Kernel (HTTP)",
            url = "http://mirrors.edge.kernel.org/pub/site/README"
        )

        TestHtmlQuery(
            text = "Kernel (HTTPS)",
            url = "https://mirrors.edge.kernel.org/pub/site/README"
        )

        TestDnsQuery(
            text = "Google (DNS)",
            domain = "google.com."
        )

        TestDnsQuery(
            text = "Non-Existent Server (DNS)",
            domain = "google.com.",
            server = "172.0.0.1"
        )
    }
}

@Composable
private fun TestHtmlQuery(text: String, url: String) {
    val coroutineScope = rememberCoroutineScope()

    fun performJsoupRequest(onRequestStarted: () -> Unit, onRequestFinished: (Boolean) -> Unit) {
        coroutineScope.launch(Dispatchers.IO) {
            val requestStartTime = System.currentTimeMillis()
            onRequestStarted()
            val conn = Jsoup
                .connect(url)
                .followRedirects(false)
                .method(Connection.Method.GET)
            try {
                val resp = conn.execute()
                val html = resp.body()
                val duration = System.currentTimeMillis() - requestStartTime

                Timber.d(
                    """
                        |dumping html, count=[${html.length}] duration=[$duration]
                        |$html
                        |done dumping html
                    """.trimMargin()
                )
                onRequestFinished(true)
            } catch (e: SocketTimeoutException) {
                Timber.e(e, "Request timed out")
                onRequestFinished(false)
            } catch (e: IOException) {
                Timber.e(e)
                onRequestFinished(false)
            } catch (e: RuntimeException) {
                Timber.e(e)
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
private fun TestDnsQuery(text: String, domain: String, server: String = "8.8.8.8") {
    val coroutineScope = rememberCoroutineScope()

    fun performDnsLookup(onRequestStarted: () -> Unit, onRequestFinished: (Boolean) -> Unit) {
        coroutineScope.launch(Dispatchers.IO) {
            val requestStartTime = System.currentTimeMillis()
            onRequestStarted()

            try {
                val queryRecord = Record.newRecord(Name.fromString(domain), Type.A, DClass.IN)
                val queryMessage = Message.newQuery(queryRecord)
                SimpleResolver(server)
                    .sendAsync(queryMessage)
                    .whenComplete { answer, e ->
                        if (e == null) {
                            val duration = System.currentTimeMillis() - requestStartTime
                            Timber.d(
                                """
                                |dumping dns, duration=[$duration]
                                |$answer
                                |done dumping dns
                            """.trimMargin()
                            )
                            onRequestFinished(true)
                        } else {
                            Timber.e(e)
                            onRequestFinished(false)
                        }
                    }
                    .toCompletableFuture()
                    .get()
            } catch (e: Exception) {
                Timber.e(e)
                onRequestFinished(false)
            }
        }
    }

    val buttonColor = remember { mutableStateOf(Color.Magenta) }

    Button(
        modifier = Modifier.fillMaxWidth(),
        colors = ButtonDefaults.buttonColors(backgroundColor = buttonColor.value),
        onClick = {
            performDnsLookup(
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