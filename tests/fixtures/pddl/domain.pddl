(define (domain delivery)
  (:requirements :strips :typing)
  (:types package location)
  (:predicates
    (at ?p - package ?l - location)
    (connected ?from - location ?to - location)
    (delivered ?p - package)
  )
  (:action deliver
    :parameters (?p - package ?from - location ?to - location)
    :precondition (and
      (at ?p ?from)
      (connected ?from ?to)
    )
    :effect (and
      (not (at ?p ?from))
      (at ?p ?to)
      (delivered ?p)
    )
  )
)
