# Beacon Verification 

To verify multiple pulses, we aggregate beacon pulses and check the single, aggregated signature, rather than verifying each one independently. In cases with more than one pulse, this provides a huge performance boost. 

First, we lay out some assumptions:
-  the list of beacon pulses have monotonically increasing round numbers
    - if anything is out of order or missing, then verification will fail
    - we should research an efficient to determine what was missing if that's the case

