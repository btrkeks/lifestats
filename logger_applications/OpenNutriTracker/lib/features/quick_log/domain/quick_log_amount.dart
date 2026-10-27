const quickLogMaxBaseAmount = 10000.0;

bool isValidQuickLogAmount(double amount) =>
    amount.isFinite && amount > 0 && amount <= quickLogMaxBaseAmount;
