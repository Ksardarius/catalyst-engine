# Catalyst Engine

![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust)
![ECS](https://img.shields.io/badge/ECS-Flecs-green)
![Physics](https://img.shields.io/badge/Physics-Rapier3D-blue)

**Catalyst Engine** is a modular, high-performance 3D game engine written in Rust. It leverages the speed of **Flecs ECS** for data management and **Rapier3D** for deterministic physics simulations.

The engine is designed with a strict separation of concerns, ensuring that rendering, physics, and game logic run in decoupled, efficient pipelines.

## 🌟 Key Features

* **Data-Driven Architecture**: Built on top of `flecs_ecs`, enabling fast query iteration and cache-friendly memory layout.
* **Advanced Physics Integration**:
    * Powered by `rapier3d`.
    * **Pipeline Split**: Distinct "Prepare" (ECS → Physics) and "Sync" (Physics → ECS) phases.
    * **Scale Handling**: Automatic baking of `GlobalTransform` scale into colliders (supporting scaled parent hierarchies).
    * **Hybrid Workflow**: Supports both Mesh-based colliders and Primitive-based colliders (Empties).
* **Asset Pipeline**:
    * glTF / GLB scene loading.
    * Automated collider generation from Blender nodes.
* **Debugging Tools**:
    * Integrated `DebugDraw3D` for visualizing physics colliders, grids, and rays.
    * Gizmo-style debug lines.

## 📦 Workspace Structure

The engine is organized as a Rust workspace with modular crates:

| Crate | Description |
| :--- | :--- |
| **`catalyst_core`** | The engine kernel. Defines `App`, the main loop, and core components (`Transform`, `GlobalTransform`, `Time`). |
| **`catalyst_physics`** | Rapier3D integration. Manages `RigidBody`, `Collider`, and synchronization systems. |
| **`catalyst_renderer`** | WGPU-based rendering backend and debug drawing resources. |
| **`catalyst_assets`** | Asset management, glTF loaders, and material definitions. |

## 🚀 Getting Started

### Prerequisites

* [Rust Toolchain](https://www.rust-lang.org/tools/install) (Stable)
* A GPU with Vulkan, Metal, or DX12 support.

### Installation

```bash
git clone [https://github.com/Ksardarius/catalyst-engine.git](https://github.com/Ksardarius/catalyst-engine.git)
cd catalyst-engine
cargo build --release
