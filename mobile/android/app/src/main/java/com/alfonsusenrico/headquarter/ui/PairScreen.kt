package com.alfonsusenrico.headquarter.ui

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import com.alfonsusenrico.headquarter.Core
import com.google.zxing.BarcodeFormat
import com.google.zxing.BinaryBitmap
import com.google.zxing.DecodeHintType
import com.google.zxing.MultiFormatReader
import com.google.zxing.NotFoundException
import com.google.zxing.PlanarYUVLuminanceSource
import com.google.zxing.common.HybridBinarizer
import kotlinx.coroutines.launch
import uniffi.hq_mobile.MobileException
import java.util.concurrent.Executors

/** Pairs with a computer from its QR code, or the code typed in (add-android-app D3). */
@Composable
fun PairScreen(
    core: Core,
    canGoBack: Boolean,
    onPaired: () -> Unit,
    onBack: () -> Unit,
) {
    val tokens = LocalTokens.current
    val context = LocalContext.current
    var typing by remember { mutableStateOf(false) }
    var typed by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var problem by remember { mutableStateOf<String?>(null) }
    var cameraAllowed by remember {
        mutableStateOf(
            ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED,
        )
    }
    var cameraRefused by remember { mutableStateOf(false) }
    val askCamera =
        rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            cameraAllowed = granted
            cameraRefused = !granted
            if (!granted) typing = true
        }
    LaunchedEffect(Unit) { if (!cameraAllowed) askCamera.launch(Manifest.permission.CAMERA) }

    fun pair(code: String) {
        if (busy) return
        busy = true
        problem = null
        val name =
            listOf(Build.MANUFACTURER, Build.MODEL)
                .distinct()
                .joinToString(" ")
                .trim()
                .ifEmpty { "Phone" }
        core.io {
            try {
                core.phone.pair(code.trim(), name)
                core.scope.launch { onPaired() }
            } catch (error: MobileException) {
                problem =
                    when (error) {
                        is MobileException.Refused -> "${error.reason} Show a new code on the computer."
                        else -> error.ownerText()
                    }
                busy = false
            }
        }
    }

    Column(
        Modifier
            .fillMaxSize()
            .statusBarsPadding()
            .imePadding()
            .navigationBarsPadding(),
    ) {
        Crumb(if (canGoBack) "Pair another computer" else "Headquarter", null, if (canGoBack) onBack else null)
        Column(
            Modifier
                .fillMaxSize()
                .verticalScroll(
                    rememberScrollState(),
                ).padding(start = 20.dp, end = 20.dp, bottom = 24.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            Text("Pair with your computer", fontSize = 22.sp, fontWeight = FontWeight.SemiBold)
            if (core.keyReplaced) {
                Text(
                    "This phone's key had to be replaced, so the computers it knew must be paired again.",
                    color = tokens.amber,
                )
            }
            // The steps say what the computer shows (refresh-phone-screens D7).
            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Step(1, "Open Headquarter on your computer.")
                Step(2, "Choose Headquarter ▸ Remote Control… ▸ Pair a Phone.")
                Step(3, if (typing) "Enter the code shown under the QR code." else "Point the camera at the code.")
            }
            if (cameraAllowed && !typing) {
                Box(
                    Modifier
                        .fillMaxWidth()
                        .aspectRatio(1f)
                        .clip(RoundedCornerShape(20.dp))
                        .semantics { contentDescription = "Camera" },
                ) {
                    Scanner(onCode = { if (it.startsWith("hqp")) pair(it) })
                    Viewfinder(Modifier.fillMaxSize(0.62f).align(Alignment.Center))
                }
            }
            if (cameraRefused) Text("The camera is off for this app, so enter the code instead.", color = tokens.muted)
            if (typing) {
                val shape = RoundedCornerShape(12.dp)
                Box(
                    Modifier
                        .fillMaxWidth()
                        .heightIn(min = 48.dp)
                        .clip(shape)
                        .background(tokens.card)
                        .border(1.dp, tokens.line, shape)
                        .padding(12.dp),
                ) {
                    if (typed.isEmpty()) Text("hqp2|…", color = tokens.muted)
                    BasicTextField(
                        value = typed,
                        onValueChange = { typed = it },
                        textStyle = TextStyle(color = tokens.text, fontFamily = Mono, fontSize = 13.sp),
                        cursorBrush = SolidColor(tokens.accent),
                        // The code must arrive exactly as shown, and Gboard never corrects a URI field.
                        keyboardOptions =
                            KeyboardOptions(
                                capitalization = KeyboardCapitalization.None,
                                autoCorrectEnabled = false,
                                keyboardType = KeyboardType.Uri,
                            ),
                        modifier = Modifier.fillMaxWidth().semantics { contentDescription = "Pairing code" },
                    )
                }
                Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
                    HqButton(
                        if (busy) "Pairing…" else "Pair",
                        onClick = { pair(typed) },
                        primary = true,
                        enabled =
                            !busy && typed.isNotBlank(),
                    )
                }
            } else {
                HqButton("Enter the code instead", onClick = { typing = true }, modifier = Modifier.fillMaxWidth())
            }
            if (busy && !typing) Text("Pairing…", color = tokens.muted)
            problem?.let {
                Text(
                    it,
                    color = tokens.danger,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
            Text(
                "The code works once and expires in 2 minutes.",
                color = tokens.muted,
                fontSize = 12.5.sp,
                textAlign = TextAlign.Center,
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}

/** A numbered step, the number on the accent-soft circle. */
@Composable
private fun Step(
    number: Int,
    text: String,
) {
    val tokens = LocalTokens.current
    Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.Top) {
        Box(
            Modifier.size(26.dp).clip(CircleShape).background(tokens.accentSoft),
            contentAlignment = Alignment.Center,
        ) { Text("$number", color = tokens.accent, fontWeight = FontWeight.Bold, fontSize = 13.sp) }
        Text(text, fontSize = 14.5.sp, modifier = Modifier.padding(top = 3.dp))
    }
}

/** Four white corner marks over the camera, where the code should sit. */
@Composable
private fun Viewfinder(modifier: Modifier) {
    Canvas(modifier) {
        val arm = 34.dp.toPx()
        val width = 4.dp.toPx()
        val inset = width / 2
        val corners =
            listOf(
                Offset(inset, inset) to Offset(1f, 1f),
                Offset(size.width - inset, inset) to Offset(-1f, 1f),
                Offset(inset, size.height - inset) to Offset(1f, -1f),
                Offset(size.width - inset, size.height - inset) to Offset(-1f, -1f),
            )
        corners.forEach { (corner, direction) ->
            drawLine(Color.White, corner, corner + Offset(arm * direction.x, 0f), width, StrokeCap.Round)
            drawLine(Color.White, corner, corner + Offset(0f, arm * direction.y), width, StrokeCap.Round)
        }
    }
}

/** The camera's view, reading QR codes with ZXing until one is found. */
@Composable
private fun Scanner(onCode: (String) -> Unit) {
    val context = LocalContext.current
    val lifecycle = LocalLifecycleOwner.current
    val executor = remember { Executors.newSingleThreadExecutor() }
    val reader =
        remember {
            MultiFormatReader().apply {
                // `hq remote pair` draws in the terminal's colours, so a dark terminal shows the
                // code inverted: light squares on dark.
                setHints(
                    mapOf(
                        DecodeHintType.POSSIBLE_FORMATS to listOf(BarcodeFormat.QR_CODE),
                        DecodeHintType.ALSO_INVERTED to true,
                    ),
                )
            }
        }
    DisposableEffect(Unit) { onDispose { executor.shutdown() } }
    AndroidView(
        factory = { viewContext ->
            val view = PreviewView(viewContext)
            val providerFuture = ProcessCameraProvider.getInstance(viewContext)
            providerFuture.addListener({
                val provider = providerFuture.get()
                val preview = Preview.Builder().build().also { it.surfaceProvider = view.surfaceProvider }
                val analysis =
                    ImageAnalysis
                        .Builder()
                        .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                        .build()
                analysis.setAnalyzer(executor) { image ->
                    val text = decode(reader, image)
                    image.close()
                    if (text != null) ContextCompat.getMainExecutor(context).execute { onCode(text) }
                }
                provider.unbindAll()
                provider.bindToLifecycle(lifecycle, CameraSelector.DEFAULT_BACK_CAMERA, preview, analysis)
            }, ContextCompat.getMainExecutor(viewContext))
            view
        },
        modifier = Modifier.fillMaxSize(),
    )
}

/** A QR code's text from the image's brightness plane, or null. */
private fun decode(
    reader: MultiFormatReader,
    image: ImageProxy,
): String? {
    val plane = image.planes[0]
    val buffer = plane.buffer
    val data = ByteArray(buffer.remaining()).also { buffer.get(it) }
    val source =
        PlanarYUVLuminanceSource(data, plane.rowStride, image.height, 0, 0, image.width, image.height, false)
    return try {
        reader.decodeWithState(BinaryBitmap(HybridBinarizer(source))).text
    } catch (_: NotFoundException) {
        null
    } finally {
        reader.reset()
    }
}
