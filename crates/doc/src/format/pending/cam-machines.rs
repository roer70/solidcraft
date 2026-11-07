//! Machines (`manufacture.machines`: each with a name, its axes as a chain from the table to the
//! tool, and a default post) and the machine a setup runs on (`manufacture.setups[*].machine`: a
//! machine's name; left out for a generic three-axis mill). Older designs need no transformation:
//! without machines every setup runs as before. The last release asks for an update instead of
//! opening a design and dropping its machines on the next save.
