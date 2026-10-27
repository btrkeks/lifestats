package com.opennutritracker.ont.opennutritracker.quicklog.widget

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF

/**
 * The rings (kcal and protein), drawn with a Canvas because Glance has no
 * arc primitive.
 *
 * Each layer is a white mask that the widget tints. A bitmap drawn in fixed
 * colours would keep them when the launcher switches to dark mode with the
 * app dead; a tint takes the launcher's day or night colour at apply time.
 */
object RingBitmap {
    /** A ring drawing in the mock's units: a [box]-unit square, [stroke] wide. */
    class Geometry(val box: Float, val stroke: Float, vararg val radii: Float)

    /** The 4×2 pair: kcal outside, protein inside. */
    val CONCENTRIC = Geometry(124f, 9f, 57.5f, 45.5f)

    /** One 4×1 status ring. */
    val MINI = Geometry(20f, 3.5f, 7f)

    /** The full circles behind the arcs, one per radius. */
    fun tracks(sizePx: Int, geometry: Geometry): Bitmap = draw(sizePx, geometry) { canvas, paint, scale ->
        for (radius in geometry.radii) canvas.drawCircle(sizePx / 2f, sizePx / 2f, radius * scale, paint)
    }

    /**
     * One arc from 12 o'clock, clockwise, round-capped. A zero fraction
     * still draws the cap as a dot, the start marker the mock shows.
     */
    fun arc(sizePx: Int, geometry: Geometry, radius: Float, fraction: Float): Bitmap =
        draw(sizePx, geometry) { canvas, paint, scale ->
            val r = radius * scale
            val c = sizePx / 2f
            val sweep = fraction.coerceIn(0f, 1f) * 360f
            if (sweep <= 0f) {
                paint.style = Paint.Style.FILL
                canvas.drawCircle(c, c - r, paint.strokeWidth / 2f, paint)
            } else {
                canvas.drawArc(RectF(c - r, c - r, c + r, c + r), -90f, sweep, false, paint)
            }
        }

    private fun draw(sizePx: Int, geometry: Geometry, block: (Canvas, Paint, Float) -> Unit): Bitmap {
        val bitmap = Bitmap.createBitmap(sizePx, sizePx, Bitmap.Config.ARGB_8888)
        val scale = sizePx / geometry.box
        val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = Color.WHITE
            style = Paint.Style.STROKE
            strokeWidth = geometry.stroke * scale
            strokeCap = Paint.Cap.ROUND
        }
        block(Canvas(bitmap), paint, scale)
        return bitmap
    }
}
