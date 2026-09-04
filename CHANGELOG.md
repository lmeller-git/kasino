# Version 0.3.0

- [BREAKING] change names of invalidation policies for `DoubleCollect`.
- [BREAKING] remove aome derives on multiple strategies.
- [CHANGED] fork on handles no longer requires exclusize access.
- [CHANGED] improve fork of various strategies to better distribute access of sub collections.
- [FIXED] fix race in `DoubleCollect` which lead to empty-linearizability violations.
- [CHANGED] rename old collection strategies and rename `Strategy::collect` to `Strategy::on_poll_fail`.
- [ADDED] add `on_offer_fail` to `Strategy`.
- [ADDED] add `offer_with_info` to `Bandit`.
- [CHANGED] cache padding for strategy stakes is now applied at storage level based on the combined padding requests of the fully resolved stake.
- [ADDED] add options for specifying padding requests of particular strategy hooks, as well as required associated type on `Hooked` for probagating padding requests upward.

# Version 0.2.0

- [BREAKING] change `inlineBandit` and `BoxedBandit` to type aliases of the now exported `Bandit`.
- [ADDED] add policies and a policy paramter to `DoubleCollect`, determining state invalidation behaviour under contention.

# Version 0.1.1

- [CHANGED] remove IndexMut bound on StorageBackend::Rebind.
