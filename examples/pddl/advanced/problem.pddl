(define (problem advanced-problem)
  (:domain advanced)
  (:requirements :strips :typing :preferences :constraints)
  (:objects
    truck1 truck2 - truck
    pkg1 pkg2 - package
    loc1 loc2 - loc
  )
  (:init
    (at truck1 loc1)
    (at pkg1 loc1)
    (over pkg1 loc1)
    (not (blocked loc2))
    (= (fuel truck1) 10)
    (= total-cost 0)
    (at 5 (blocked loc1))
  )
  (:goal
    (and
      (delivered pkg1)
      (preference (visited loc2))
      (within 10 (visited loc2))
    )
  )
  (:constraints
    (and
      (always (not (blocked loc2)))
      (sometime (visited loc2))
      (at-most-once (blocked loc1))
      (sometime-before (visited loc1) (visited loc2))
      (sometime-after (visited loc2) (visited loc1))
      (always-within 5 (visited loc1) (visited loc2))
      (hold-during 1 3 (clear loc2))
      (hold-after 4 (visited loc2))
    )
  )
  (:metric maximize (+ (total-time) (total-cost) (- 5) (/ 10 2)))
  (:unknown (skip me))
)
