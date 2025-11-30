---
description: Scientific Debugging Method - experiment-driven bug fixing
---

# Scientific Debugging Method

This command guides you through a 7-step, experiment-driven approach to finding and fixing bugs. Replace guesswork with a repeatable methodology.

## Usage

```
/debug                                    # Start interactively
/debug login fails after OAuth redirect   # Pre-fill with bug description
```

## Why This Works

Debugging is a scientific process: observe symptoms, form hypotheses, design experiments, and validate. This methodology prevents:
- Shotgun debugging (changing random things hoping something works)
- Fixing symptoms instead of root causes
- Introducing new bugs while fixing old ones
- Forgetting to add regression tests

---

## Instructions

### 1. Initialize the Debug Session

First, set up the todo list to track progress through all 7 steps:

```
TodoWrite:
1. Define problem (expected vs actual + evidence) - pending
2. Form hypothesis (testable "if X then Y when Z") - pending
3. Design experiment (reproducible test) - pending
4. Reproduce bug (confirm failure) - pending
5. Apply minimal fix (smallest change) - pending
6. Verify fix (test passes + regressions) - pending
7. Reflect & prevent (root cause + improvements) - pending
```

### 2. Process the Bug Description

If `$ARGUMENTS` was provided, use it to pre-fill step 1. Otherwise, ask the user to describe the bug.

---

## The 7 Steps

### Step 1: Define the Problem

**Goal**: Clearly state what should happen vs. what actually happens.

**Gather from user**:
- **Expected behavior**: "What should happen?"
- **Actual behavior**: "What actually happens?"
- **Evidence**: Logs, error messages, screenshots, inputs that trigger it

**For API-related bugs**: If the issue involves external API calls (Jira, Asana, etc.), consider asking the user to make a real API call with their account and provide the raw response. Real API data often reveals:
- Unexpected response structures
- Missing or renamed fields
- Edge cases in the data
- Authentication/permission issues

**Template**:
```
EXPECTED: [what should happen]
ACTUAL: [what happens instead]
EVIDENCE: [logs/errors/screenshots]
API_RESPONSE: [if applicable, actual API response data]
```

**Mark complete when**: You have concrete, observable symptoms documented.

---

### Step 2: Form a Hypothesis

**Goal**: Create a single, testable theory about the cause.

**Template**:
```
If [suspected cause] is the problem,
then [observable effect] will happen
when [specific test action].
```

**Guidelines**:
- Keep it small enough to test quickly
- One hypothesis at a time
- Be specific: "the OAuth token expires" not "something is wrong with auth"

**Example**:
```
If the session cookie is not being set after OAuth callback,
then the user's session will be empty
when I check req.session immediately after redirect.
```

**Mark complete when**: You have a concrete, falsifiable hypothesis.

---

### Step 3: Design an Experiment

**Goal**: Decide how to prove the hypothesis right or wrong.

**Ask**:
- "How will we prove/disprove this hypothesis?"
- "Can we write an automated test for this?"
- "If not, what's the minimal manual reproduction script?"

**Prefer**: Automated tests (unit, integration) that can become regression tests.
**Accept**: Reproducible manual steps if automation is impractical for now.

**Output**: A test file or documented manual steps.

**Mark complete when**: You have a concrete experiment ready to run.

---

### Step 4: Reproduce the Bug

**Goal**: Run the experiment and confirm the bug exists.

**Actions**:
1. Run the experiment (test or manual steps)
2. Confirm it **fails** as expected (the bug appears)
3. If you can't reliably reproduce it, **stop** - you're not ready to fix yet

**For API integration issues**: If reproducing the bug requires understanding the actual API response format, ask the user to make a real API call and share the response. This is especially helpful when:
- Mocked test data might not match real API behavior
- You need to see the exact field names, types, or structures
- Error responses need analysis
- You suspect API documentation is outdated or incorrect

**Important**: If reproduction fails, revisit Step 1 (gather more evidence) or Step 2 (refine hypothesis).

**Mark complete when**: The experiment reliably demonstrates the bug.

---

### Step 5: Apply the Minimal Fix

**Goal**: Change the smallest possible piece of code.

**Rules**:
- One change at a time
- Don't refactor unrelated code
- Don't add features while fixing bugs
- Keep the diff small and reviewable

**Ask**: "What is the absolute minimum change that could fix this?"

**Mark complete when**: You've made a focused, minimal code change.

---

### Step 6: Verify the Fix

**Goal**: Confirm the fix works without breaking other things.

**Actions**:
1. Re-run the same experiment from Step 3 → should now **pass**
2. Run related regression tests
3. Check critical user flows that might be affected

**If the test still fails**: The hypothesis was wrong or the fix is incomplete. Return to Step 2.

**Mark complete when**: Experiment passes and no regressions found.

---

### Step 7: Reflect and Prevent Recurrence

**Goal**: Ensure this class of bug doesn't happen again.

**Questions to answer**:
- **Why did this bug appear?** (technical root cause)
- **Why wasn't it caught earlier?** (process root cause)
- **What can prevent similar bugs?**

**Actions to consider**:
- Add the experiment as a permanent regression test
- Improve error messages or logging
- Add monitoring or alerts
- Update documentation
- Discuss in code review / retro

**Mark complete when**: You've captured lessons learned and added preventive measures.

---

## Flexibility Guidelines

This methodology is **guided but flexible**:

- **Recommend** following steps in order
- **Allow** skipping if the user has strong justification (e.g., "I already know the root cause from logs")
- **Acknowledge** skips: "Skipping hypothesis formation. Note: if the fix doesn't work, we may return to form a hypothesis."
- **Don't block** progress, but remind of methodology benefits

If user wants to skip:
```
"Acknowledged - skipping [step]. If we hit a dead end, we can revisit this step.
Proceeding to [next step]..."
```

---

## Example Session

**User**: `/debug OAuth login redirects to blank page`

**Step 1 - Define**:
```
EXPECTED: After Jira OAuth, user redirects to dashboard
ACTUAL: User sees blank page after OAuth callback
EVIDENCE: Browser shows 302 to /dashboard, but page is empty. No errors in console.
```

*(Note: For API-related bugs, the model might ask: "Can you make a test OAuth callback request and share the raw response? This will help me see if the token structure or user data fields match what we expect.")*

**Step 2 - Hypothesis**:
```
If the session is not persisted before redirect,
then the auth state will be lost
when the browser follows the redirect to /dashboard.
```

**Step 3 - Experiment**:
Write integration test that:
1. Simulates OAuth callback with valid token
2. Follows redirect
3. Asserts session contains user data

**Step 4 - Reproduce**:
Test fails - session is empty after redirect. Bug confirmed.

**Step 5 - Fix**:
Add `session.save()` before redirect in OAuth callback handler.

**Step 6 - Verify**:
Test passes. Manual test confirms login works. Existing auth tests pass.

**Step 7 - Reflect**:
- Root cause: Async session save wasn't awaited
- Prevention: Add test for session persistence after OAuth
- Docs: Note session save requirement in CLAUDE.md auth section

---

Begin by initializing the todo list and asking about the bug (or processing `$ARGUMENTS` if provided).
