# **Catalyst Input System — Architecture Overview**

This document outlines the full architecture of the Catalyst Input System.  
The system is designed in **seven incremental phases**, each building on the previous one.

Only the first phases are implemented; the rest are planned for future development.

## **Phase 1 — Raw Input Collection *(implemented)***

Collects unprocessed hardware input from all supported devices:

* **keyboard keys**  
* **mouse buttons**  
* **mouse position**  
* **mouse delta**  
* **scroll wheel**  
* **gamepad buttons**  
* **gamepad axes**

This phase provides a backend‑agnostic layer that normalizes device events into a unified internal representation.

## **Phase 2 — Logical Input Mapping *(implemented)***

Maps physical inputs to logical engine‑level concepts:

* **actions** (Pressed, Held, Released)  
* **axes** (MoveX, LookY, etc.)  
* **bindings** (Physical → Logical)  
* **device‑agnostic gameplay input**

Gameplay systems read logical input only, never raw hardware events.

## **Phase 3 — Input Contexts *(implemented)***

Introduces context‑aware input:

* **context stack** (push/pop)  
* **UI vs Gameplay separation**  
* **vehicle mode**  
* **editor mode**  
* **priority rules**

Only bindings from active contexts are processed, enabling clean modal behavior.

# **Missing Phases**

The following phases are **planned but not yet implemented**.

They are included here for architectural completeness.

## **Phase 4 — Modifiers & Composite Bindings *(missing)***

Adds expressive input shaping and transformation:

* **deadzones**  
* **sensitivity**  
* **inversion**  
* **curves** (linear, exponential, power)  
* **WASD → MoveX/MoveY**  
* **mouse \+ gamepad → unified LookX/LookY**  
* **composite axes**  
* **chords** (e.g., Shift \+ W)

This phase makes input *feel* correct and consistent across devices.

## **Phase 5 — Input Buffering & State History *(missing)***

Adds temporal logic for responsive gameplay:

* **action buffering**  
* **coyote time**  
* **tap vs hold detection**  
* **double‑tap detection**  
* **combo sequences**

Essential for platformers, action games, and polished controls.

## **Phase 6 — Multi‑Device & Multi‑Player Routing *(missing)***

Adds support for multiple simultaneous devices:

* **multiple keyboards**  
* **multiple mice**  
* **multiple gamepads**  
* **player‑device assignment**  
* **hot‑swapping devices**

Required for local multiplayer and advanced setups.

## **Phase 7 — Rebinding & User Configuration *(missing)***

Adds user‑facing configuration:

* **action rebinding**  
* **axis rebinding**  
* **saving/loading profiles**  
* **UI for rebinding**  
* **conflict resolution**

This phase completes the system and makes it player‑friendly.

# **Summary**

| Phase | Status |
| ----- | ----- |
| **1\. Raw Input Collection** | Implemented |
| **2\. Logical Input Mapping** | Implemented |
| **3\. Input Contexts** | Implemented |
| **4\. Modifiers & Composite Bindings** | **Missing** |
| **5\. Input Buffering & State History** | **Missing** |
| **6\. Multi‑Device & Multi‑Player Routing** | **Missing** |
| **7\. Rebinding & User Configuration** | **Missing** |

