//! A setup that stays in an earlier setup's clamping and is only turned with the A axis
//! (`manufacture.setups[*].same_clamping`: true, beside its `stock_from`; left out when the part
//! is clamped again). Older designs need no transformation: without it every setup is a new
//! clamping, as all were. The last release asks for an update instead of opening a design that
//! has it, which would otherwise ask for the part to be clamped again.
