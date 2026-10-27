package com.opennutritracker.ont.opennutritracker.quicklog.widget

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF

/**
 * The two concentric rings (kcal outside, protein inside), drawn with a
 * Canvas because Glance has no arc primitive.
 *
 * Each layer is a white mask that the widget tints. A bitmap drawn in fixed
 * colours would keep them when the launcher switches to dark mode with the
 * app dead; a tint takes the launcher's day or night colour at apply time.
 *
 * Geometry is the mock's: a 124-unit box, radii 57.5 and 45.5, stroke 9.
 */
object RingBitmap {
    private const val BOX = 124f
    private const val STROKE = 9f
    const val OUTER_RADIUS = 57.5f
    const val INNER_RADIUS = 45.5f

    /** Both tracks: the full circles behind the arcs. */
    fun tracks(sizePx: Int): Bitmap = draw(sizePx) { canvas, paint, scale ->
        canvas.drawCircle(sizePx / 2f, sizePx / 2f, OUTER_RADIUS * scale, paint)
        canvas.drawCircle(sizePx / 2f, sizePx / 2f, INNER_RADIUS * scale, paint)
    }

    /**
     * One arc from 12 o'clock, clockwise, round-capped. A zero fraction
     * still draws the cap as a dot, the start marker the mock shows.
     */
    fun arc(sizePx: Int, radius: Float, fraction: Float): Bitmap = draw(sizePx) { canvas, paint, scale ->
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

    private fun draw(sizePx: Int, block: (Canvas, Paint, Float) -> Unit): Bitmap {
        val bitmap = Bitmap.createBitmap(sizePx, sizePx, Bitmap.Config.ARGB_8888)
        val scale = sizePx / BOX
        val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = Color.WHITE
            style = Paint.Style.STROKE
            strokeWidth = STROKE * scale
            strokeCap = Paint.Cap.ROUND
        }
        block(Canvas(bitmap), paint, scale)
        return bitmap
    }
}
