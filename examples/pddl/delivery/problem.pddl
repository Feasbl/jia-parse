(define (problem delivery-1)
  (:domain delivery)
  (:objects
    pkg1 - package
    depot customer - location
  )
  (:init
    (at pkg1 depot)
    (connected depot customer)
  )
  (:goal (and (delivered pkg1)))
)
