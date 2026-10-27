// In GoalFields statisticType switch:
// case 'TimedAmounts': return <AmountFields form={form} measure={measure} compact={compact}/>;
// Make the function return type explicitly ReactNode (and an inline never assertion
// after the exhaustive switch) so the next type cannot silently render no form.
// Existing hasInlineFields permits TimedAmounts and is correct for its single amount goal.
