(in-package "PDDL")

(define (domain advanced)
  (:requirements
    :strips :typing :negative-preconditions :disjunctive-preconditions
    :equality :existential-preconditions :universal-preconditions
    :quantified-preconditions :conditional-effects :fluents
    :numeric-fluents :adl :durative-actions :duration-inequalities
    :timed-initial-literals :preferences :constraints :action-costs
    :goal-utilities :derived-predicates :domain-axioms :unknown-requirement
  )
  (:types truck package loc - object fragile - package either-holder - (either truck package))
  (:constants depot hub - loc)
  (:predicates
    (at ?x - (either truck package) ?l - loc)
    (over ?p - package ?l - loc)
    (clear ?l - loc)
    (loaded ?p - package ?t - truck)
    (fragile ?p - package)
    (ready)
    (visited ?l - loc)
    (delivered ?p - package)
    (blocked ?l - loc)
  )
  (:functions
    (fuel ?t - truck) - number
    (distance ?from - loc ?to - loc)
    (total-cost)
  )
  (:derived (reachable ?from - loc ?to - loc)
    (or
      (= ?from ?to)
      (and (clear ?from) (clear ?to))
    )
  )
  (:action load
    :parameters (?p - package ?t - truck ?l - loc)
    :vars (?helper - truck)
    :precondition
      (and
        (at ?p ?l)
        (at ?t ?l)
        (not (blocked ?l))
        (or (clear ?l) (ready))
        (imply (fragile ?p) (clear ?l))
        (exists (?other - loc) (clear ?other))
        (forall (?q - package) (not (= ?q ?t)))
        (preference prefer-clear (clear ?l))
        (= ?p ?p)
        (= (fuel ?t) (+ 1 2 3))
        (>= (fuel ?t) (/ (* 12 2) (- 8 2)))
        (at ?t ?l)
        (over ?p ?l)
      )
    :effect
      (and
        (not (at ?p ?l))
        (loaded ?p ?t)
        (forall (?q - package) (when (fragile ?q) (visited ?l)))
        (assign (fuel ?t) (- (fuel ?t) 1))
        (increase (total-cost) (distance ?l depot))
        (decrease (fuel ?t) 2)
        (scale-up (fuel ?t) 2)
        (scale-down (fuel ?t) 2)
      )
    :unknown-section (nested (value))
  )
  (:durative-action drive
    :parameters (?t - truck ?from - loc ?to - loc)
    :duration (and
      (>= ?duration (+ 1 2 3))
      (<= ?duration (- 10))
    )
    :condition (and
      (at start (at ?t ?from))
      (over all (clear ?to))
      (at end (not (blocked ?to)))
    )
    :effect (and
      (at start (not (at ?t ?from)))
      (at start (at ?t ?to))
      (at end (visited ?to))
      (at end (increase (total-cost) ?duration))
    )
    :ignored (nested value)
  )
  (:mystery (skip (this section)))
)
