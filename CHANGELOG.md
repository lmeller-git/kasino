# Version 0.3.0

- [BREAKING] change names of invalidation policies for `DoubleCollect`.
- [BREAKING] remove aome derives on multiple strategies.
- [CHANGED] fork on handles no longer requires exclusize access.
- [CHANGED] improved fork of various strategies to better distribute access of sub collections.
- [FIXED] fixed race in `DoubleCollect` which lead to empty-linearizability violations.

# Version 0.2.0

- [BREAKING] change `inlineBandit` and `BoxedBandit` to type aliases of the now exported `Bandit`.
- [ADDED] added policies and a policy paramter to `DoubleCollect`, determining state invalidation behaviour under contention.

# Version 0.1.1

- [CHANGED] remove IndexMut bound on StorageBackend::Rebind.
