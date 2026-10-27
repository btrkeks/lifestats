package com.opennutritracker.ont.opennutritracker.quicklog

import org.json.JSONException
import org.json.JSONObject

/**
 * The widget snapshot Dart publishes (wire v1). Dart's golden fixture,
 * test/fixtures/quick_log_snapshot_v1.json, is parsed by QuickLogModelTest,
 * so a change on either side fails a test. Kotlin reads only what it draws;
 * every string arrives formatted and localized.
 */
sealed interface WidgetSnapshot {
    /** Nothing published yet: a fresh install, or the widget just placed. */
    data object None : WidgetSnapshot

    data object Unreadable : WidgetSnapshot

    data class NeedsSetup(val title: String) : WidgetSnapshot

    data class Ready(
        val profileId: String,
        val validUntilMs: Long,
        val status: Status,
        val labels: Labels,
        val rows: List<Row>,
    ) : WidgetSnapshot

    companion object {
        const val VERSION = 1

        /** Never throws: anything malformed is [Unreadable]. */
        fun parse(json: String?): WidgetSnapshot {
            if (json == null) return None
            return try {
                val root = JSONObject(json)
                if (root.getInt("v") != VERSION) return Unreadable
                when (root.getString("kind")) {
                    "needsSetup" -> NeedsSetup(root.getString("title"))
                    "ready" -> readReady(root)
                    else -> Unreadable
                }
            } catch (e: JSONException) {
                Unreadable
            }
        }

        private fun readReady(root: JSONObject): Ready {
            val status = root.getJSONObject("status")
            val labels = root.getJSONObject("labels")
            val rows = root.getJSONArray("rows")
            return Ready(
                profileId = root.getString("profileId"),
                validUntilMs = root.getLong("validUntilMs"),
                status = Status(
                    energyValueText = status.getString("energyValueText"),
                    energyLabelText = status.getString("energyLabelText"),
                    energyFraction = status.getDouble("energyFraction").toFloat(),
                    proteinText = status.getString("proteinText"),
                    proteinCompactText = status.getString("proteinCompactText"),
                    proteinFraction = status.getDouble("proteinFraction").toFloat(),
                ),
                labels = Labels(
                    scanProduct = labels.getString("scanProduct"),
                    searchFood = labels.getString("searchFood"),
                    undo = labels.getString("undo"),
                    emptyTitle = labels.getString("emptyTitle"),
                    emptyBody = labels.getString("emptyBody"),
                ),
                rows = List(rows.length()) { i ->
                    val row = rows.getJSONObject(i)
                    Row(
                        foodKey = row.getString("foodKey"),
                        title = row.getString("title"),
                        amountText = row.getString("amountText"),
                        energyText = row.getString("energyText"),
                        pillText = row.getString("pillText"),
                        chipText = row.getString("chipText"),
                        baseAmount = row.getDouble("baseAmount"),
                        unit = row.getString("unit"),
                        pickerJson = row.getJSONObject("picker").toString(),
                    )
                },
            )
        }
    }
}

data class Status(
    val energyValueText: String,
    val energyLabelText: String,
    val energyFraction: Float,
    val proteinText: String,
    val proteinCompactText: String,
    val proteinFraction: Float,
)

data class Labels(
    val scanProduct: String,
    val searchFood: String,
    val undo: String,
    val emptyTitle: String,
    val emptyBody: String,
)

data class Row(
    val foodKey: String,
    val title: String,
    val amountText: String,
    val energyText: String,
    val pillText: String,
    val chipText: String,
    val baseAmount: Double,
    val unit: String,
    /** Opaque here; handed to the amount picker as it came. */
    val pickerJson: String,
)

/** The "✓ Added · +30 g" row with its Undo, shown after Dart acked a log. */
data class RecentAction(
    val opId: String,
    val foodKey: String,
    val profileId: String,
    val addedText: String,
    val shownAtMs: Long,
    val expiresAtMs: Long,
) {
    fun toJson(): String = JSONObject()
        .put("opId", opId)
        .put("foodKey", foodKey)
        .put("profileId", profileId)
        .put("addedText", addedText)
        .put("shownAtMs", shownAtMs)
        .put("expiresAtMs", expiresAtMs)
        .toString()

    companion object {
        fun parse(json: String?): RecentAction? {
            if (json == null) return null
            return try {
                val root = JSONObject(json)
                RecentAction(
                    opId = root.getString("opId"),
                    foodKey = root.getString("foodKey"),
                    profileId = root.getString("profileId"),
                    addedText = root.getString("addedText"),
                    shownAtMs = root.getLong("shownAtMs"),
                    expiresAtMs = root.getLong("expiresAtMs"),
                )
            } catch (e: JSONException) {
                null
            }
        }
    }
}

/** Everything one render needs, merged from the store's three sources. */
data class WidgetModel(
    val snapshot: WidgetSnapshot,
    val recentAction: RecentAction?,
    val inFlight: Set<String>,
) {
    /**
     * The action to draw at [nowMs], or null once it expired. Checked on
     * every render, so a late expiry job only delays the repaint.
     */
    fun visibleAction(nowMs: Long): RecentAction? =
        recentAction?.takeIf {
            it.expiresAtMs > nowMs && (snapshot as? WidgetSnapshot.Ready)?.profileId == it.profileId
        }

    fun row(foodKey: String): Row? =
        (snapshot as? WidgetSnapshot.Ready)?.rows?.firstOrNull { it.foodKey == foodKey }
}
