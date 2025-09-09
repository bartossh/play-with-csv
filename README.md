# Play with CSV

[![Security Audit](https://github.com/bartossh/play-with-csv/actions/workflows/audit.yml/badge.svg)](https://github.com/bartossh/play-with-csv/actions/workflows/audit.yml)

[![PR Checks](https://github.com/bartossh/play-with-csv/actions/workflows/pre-checks.yml/badge.svg)](https://github.com/bartossh/play-with-csv/actions/workflows/pre-checks.yml)

## Key part description.

### Ladger module

Brain of the account processing, makes sure all the transactins are properly assigned and accounted for the client.

### Processor module

This module is responsible for managing the data flow.

### Models module

Contains logic that is applied to the models such as ClientBalance and Transaction.
Contains serialization logic for the models and all other actions avaliable for the moodel.

## Solution

Solutions is naive (assumes single thread and no async await) and is failing at corner cases such as:
 - What if deposited after lock - I assumend it is good to reject transaction.
 - What if chargeback exceed hold value - I assumend it is good to reject transaction.
 - What if chargeback exceed hold value but avaliable amount is enough - I assumend it is good to reject transaction.
 - What if transaction id for deposit or withdrawal is repeating - I assumend it is good to reject transaction and store it in the vector of rejected transactions - no use case for that vector now, just an example that we can deal with it later keeping the record of rejected transactions.
  - What if we would like to revisit transactions in the future - I stored them in the vector of historical ordereded transactions - even if there is no purpose for this yet.

Why no async await or multithreading?

I decided to not use async await or threading to simplify the problem and until there is no need to process very large amount of data, there will be no real gain in processing performance, it might even make things underperform.

But if there will be a need to process very large amount of data I would allow myself to create multithreaded processing with Arc + Mutex lock on the ClientBalance level. There will be still posibility that transaction might be processed out of order, for example when client A sends founds to client B (trx 0) and client B (having zero balance) sends funds to C (trx 1) and trx 1 comes before trx 0.
I would need to give it more thought how to handle this case and research more about it.

## Testing

For unit test simply run:

```
cargo test
```

**This task was great fun to solve!**
