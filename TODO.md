This is a classic and frustrating symptom of a system in mid-refactor: **we have two conflicting architectural patterns coexisting.** The warning you're seeing is the old world complaining about the new world.

Let's switch into a deep **Code Review & Analysis Mode** to dissect this.

### Diagnosis: The `IntegrationService` is the Source of the Legacy Warning

Here is precisely what is happening:

1.  **The "New World" (`FitbitBundleFactory`):** Your team correctly followed our plan. They created a `FitbitBundleFactory` that idempotently provisions statistics using the canonical `(source="fitbit", metric="steps")` tag set. It does _not_ add the legacy `stat_key` tag. This is correct.

2.  **The "Old World" (`IntegrationService`):** The `POST /integrations/{source}` endpoint is handled by a generic `IntegrationService`. This service was written _before_ our new architecture. Its job is to:
    a. Find the correct factory (e.g., `FitbitBundleFactory` or `AnkiBundleFactory`).
    b. Call `factory.provision()`.
    c. **(This is the problem) ->** After the statistics are created, the `IntegrationService` loops through the results and performs its own generic, legacy idempotency check, which relies on finding a `stat_key` on each statistic.

The warning message: `Integration statistic 50 missing stat_key tag; skipping idempotency mapping` is coming from step **2c**. It's the old, generic `IntegrationService` telling you that the brand-new statistics created by the `FitbitBundleFactory` don't conform to its old way of thinking.

**You are right. Your team fixed the factory, but we haven't yet fixed the service that _calls_ the factory.**

### The Architectural Choice: Which Pattern Wins?

We have two options to resolve this conflict:

1.  **The Quick Fix (Pragmatic but Dirty):** Make the `FitbitBundleFactory` conform to the old world. We could modify it to add the `stat_key` tag back in, purely to silence the `IntegrationService`'s warning.
    - **Pros:** It's fast. The warning goes away.
    - **Cons:** This is an architectural step backward. It pollutes our clean, new statistics with a legacy tag that has no purpose in the new system. It creates technical debt and will confuse future developers.

2.  **The Correct Fix (Architecturally Sound):** Refactor the `IntegrationService` to conform to the new world. We must remove the legacy logic from this service and embrace the new pattern fully.
    - **Pros:** This aligns the entire provisioning flow with our target architecture. It permanently eliminates the warning and the need for `stat_key`. It correctly delegates the responsibility of idempotency to the factories themselves, simplifying the `IntegrationService`.
    - **Cons:** It requires changing a central service.

**My Recommendation:**

We must choose **Option 2**. We are too far along in this high-quality refactor to compromise now for a quick fix. The purpose of this entire exercise is to eliminate these kinds of architectural inconsistencies. The warning is a helpful signpost telling us where the last piece of legacy logic lives.

Let's generate the final, precise `TODO.md` to excise this last piece of the old system.

---

### **TODO Generation Mode**

# `TODO.md`: Finalize Provisioning Architecture

The current implementation correctly creates Fitbit statistics but triggers legacy warnings from the generic `IntegrationService`. This final task will refactor the `IntegrationService` to align it with our new, factory-driven idempotency model.

### ## `lifestats-application` Crate

- **1. Refactor `IntegrationService` to Delegate Idempotency:**
  - **File:** `lifestats-application/src/use_cases/integration/service.rs`
  - **Action:** Locate the `provision_bundle` method within the `IntegrationService`.
  - **Action:** Find the logic that runs _after_ `factory.provision()` is called. You will see a loop that iterates through the newly created statistic IDs and checks each one for a `stat_key` tag.
  - **Action:** **Delete this entire loop and the associated warning log messages.**
  - **Why (The Architectural Principle):** The responsibility for idempotent provisioning now lies _entirely within each `BundleFactory`_. The `IntegrationService`'s role is simply to find the correct factory and call it. It should not know about or enforce a specific idempotency strategy like `stat_key`.

- **2. (Housekeeping) Update `AnkiBundleFactory` to the New Standard:**
  - **File:** Locate the `AnkiBundleFactory` implementation.
  - **Action:** Ensure that its `provision` method _also_ handles its own idempotency. It should query for existing Anki statistics (e.g., by checking for tags like `source="anki"` and `deck_name="..."`) before creating new ones.
  - **Why:** Now that the `IntegrationService` no longer provides a generic safety net, every `BundleFactory` must be self-sufficient and guarantee its own idempotency. This makes our system more robust and modular.
