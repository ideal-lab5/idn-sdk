# Economic Testing Approach

The goal is to gather real metrics on how much subscriptions pay for their subscriptions under a simplified 
"every block dispatch" model. Here, we ignore frequency completely (and ignore idle fees). 
The general idea is to test many different subscriptions with various lifetimes, then determine how much 
each only actually paid to the IDN Treasury during its lifetime.
The flow will be: 
For i in range(N) 
  (1) quote the number of credits required for a subscription given N (N*credit_dispatch_rate)
  (2) create a subscription with lifetime N
  (3) run the subscription down until it is finalized (we precompute how much time this takes)
  (4) check the treasury account, verify how much was paid through the subscription's lifetime
  (5) cleanup/reset for next test


We are currently charging dispatch=idle=1
