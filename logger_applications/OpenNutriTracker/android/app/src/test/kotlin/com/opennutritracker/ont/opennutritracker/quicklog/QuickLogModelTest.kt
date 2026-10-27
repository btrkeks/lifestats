package com.opennutritracker.ont.opennutritracker.quicklog

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Test
import java.io.File

class QuickLogModelTest {
    /** Dart's golden: the builder test asserts it, this test parses it. */
    private val fixture = File("../../test/fixtures/quick_log_snapshot_v1.json").readText()

    @Test
    fun parsesTheSharedFixture() {
        val snapshot = WidgetSnapshot.parse(fixture) as WidgetSnapshot.Ready
        assertEquals("default", snapshot.profileId)
        assertEquals(1793145600000L, snapshot.validUntilMs)
        assertEquals("2,232", snapshot.status.energyValueText)
        assertEquals("kcal left", snapshot.status.energyLabelText)
        assertEquals(0.07f, snapshot.status.energyFraction)
        assertEquals("30/150 g", snapshot.status.proteinText)
        assertEquals(0.2f, snapshot.status.proteinFraction)
        assertEquals("Undo", snapshot.labels.undo)
        assertEquals(listOf("Speisequark", "Protein Shake"), snapshot.rows.map { it.title })
        val shake = snapshot.rows[1]
        assertEquals("name:protein shake", shake.foodKey)
        assertEquals("1 serving", shake.amountText)
        assertEquals("+ 40 g", shake.pillText)
        assertEquals(60.0, shake.baseAmount, 0.0)
        assertEquals("serving", shake.unit)
        assertEquals("Protein Shake", JSONObject(shake.pickerJson).getString("title"))
    }

    @Test
    fun malformedOrUnknownVersionIsUnreadable() {
        for (json in listOf("", "{", "[]", """{"v":2,"kind":"ready"}""", """{"v":1,"kind":"ready"}""")) {
            assertSame(json, WidgetSnapshot.Unreadable, WidgetSnapshot.parse(json))
        }
        assertSame(WidgetSnapshot.None, WidgetSnapshot.parse(null))
        assertEquals(
            WidgetSnapshot.NeedsSetup("OpenNutriTracker"),
            WidgetSnapshot.parse("""{"v":1,"kind":"needsSetup","title":"OpenNutriTracker"}"""),
        )
    }

    @Test
    fun anExpiredActionIsNotDrawn() {
        val action = RecentAction("op", "name:quark", "default", "Added · +30 g", 1_000, 6_000)
        val model = WidgetModel(WidgetSnapshot.None, action, emptySet())
        assertEquals(action, model.visibleAction(5_999))
        assertNull(model.visibleAction(6_000))
        assertEquals(action, RecentAction.parse(action.toJson()))
        assertNull(RecentAction.parse("{}"))
    }
}
