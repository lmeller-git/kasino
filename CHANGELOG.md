# Version 0.2.0

- [BREAKING] change `inlineBandit` and `BoxedBandit` to type aliases of the now exported `Bandit`.
- [ADDED] added policies and a policy paramter to `DoubleCollect`, determining state invalidation behaviour under contention.

# Version 0.1.1

- [CHANGED] remove IndexMut bound on StorageBackend::Rebind.
