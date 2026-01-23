bl_info = {
    "name": "Catalyst Physics Definitions",
    "author": "Catalyst Engine",
    "version": (1, 0),
    "blender": (3, 0, 0),
    "location": "Properties > Object > Catalyst Physics",
    "description": "UI for setting Catalyst Engine physics properties (GLTF Extras)",
    "category": "Game Engine",
}

import bpy

# -------------------------------------------------------------------
# 1. Update Functions (Sync UI -> Custom Properties)
# -------------------------------------------------------------------
# This function runs whenever you change a value in the UI.
# It writes the value directly to the object's ["custom_properties"] dict
# using the exact key names your Rust struct expects.

def update_physics_props(self, context):
    obj = context.object
    if not obj: return

    # Helper: Set property if valid, otherwise delete it so it's None in Rust
    def sync_prop(prop_name, value, condition=True):
        if condition:
            obj[prop_name] = value
        else:
            if prop_name in obj:
                del obj[prop_name]

    # --- ENUMS (Body & Shape) ---
    # We map the UI 'NONE' selection to removing the key entirely
    sync_prop("physics_body", self.cat_phys_body, self.cat_phys_body != 'NONE')
    sync_prop("physics_shape", self.cat_phys_shape, self.cat_phys_shape != 'NONE')

    # --- CONDITIONALS ---
    # Only export mass/damping if it's actually a dynamic body
    is_dynamic = self.cat_phys_body == 'dynamic'
    has_physics = self.cat_phys_body != 'NONE' or self.cat_phys_shape != 'NONE'

    sync_prop("physics_mass", self.cat_phys_mass, is_dynamic)
    sync_prop("physics_gravity_scale", self.cat_phys_gravity_scale, is_dynamic)
    sync_prop("physics_linear_damping", self.cat_phys_linear_damping, is_dynamic)
    sync_prop("physics_angular_damping", self.cat_phys_angular_damping, is_dynamic)

    # These apply to everything (Static triggers, etc.)
    sync_prop("physics_is_trigger", self.cat_phys_is_trigger, has_physics)
    sync_prop("physics_layer", self.cat_phys_layer, has_physics)
    sync_prop("physics_mask", self.cat_phys_mask, has_physics)
    
    # Material string (only if not empty)
    sync_prop("physics_material", self.cat_phys_material, has_physics and len(self.cat_phys_material) > 0)


# -------------------------------------------------------------------
# 2. Property Group (The UI State Storage)
# -------------------------------------------------------------------
class CatalystPhysicsSettings(bpy.types.PropertyGroup):
    
    # Maps directly to your Rust 'PhysicsBody' enum
    cat_phys_body: bpy.props.EnumProperty(
        name="Body Type",
        description="Type of the Rigid Body",
        items=[
            ('NONE', "None", "No physics logic"),
            ('static', "Static", "Fixed object (Walls, Floor)"),
            ('dynamic', "Dynamic", "Moving object with mass"),
            ('kinematic', "Kinematic", "Moved by code/animation only"),
        ],
        default='NONE',
        update=update_physics_props
    )

    # Maps directly to your Rust 'PhysicsShape' enum
    cat_phys_shape: bpy.props.EnumProperty(
        name="Collider Shape",
        description="Shape of the collision volume",
        items=[
            ('NONE', "None", "No collider"),
            ('box', "Box", "Cuboid shape"),
            ('sphere', "Sphere", "Spherical shape"),
            ('capsule', "Capsule", "Capsule shape"),
            ('convex', "Convex Hull", "Simplified mesh wrapper"),
            ('mesh', "Trimesh", "Exact mesh (Static only)"),
        ],
        default='NONE',
        update=update_physics_props
    )

    # Numeric fields matching 'PhysicsExtras'
    cat_phys_mass: bpy.props.FloatProperty(name="Mass", default=1.0, min=0.001, update=update_physics_props)
    cat_phys_gravity_scale: bpy.props.FloatProperty(name="Gravity Scale", default=1.0, update=update_physics_props)
    cat_phys_linear_damping: bpy.props.FloatProperty(name="Linear Damping", default=0.0, min=0.0, update=update_physics_props)
    cat_phys_angular_damping: bpy.props.FloatProperty(name="Angular Damping", default=0.0, min=0.0, update=update_physics_props)
    
    cat_phys_is_trigger: bpy.props.BoolProperty(name="Is Trigger", default=False, update=update_physics_props)
    cat_phys_layer: bpy.props.IntProperty(name="Layer", default=1, min=0, update=update_physics_props)
    cat_phys_mask: bpy.props.IntProperty(name="Mask", default=1, min=0, update=update_physics_props)
    
    cat_phys_material: bpy.props.StringProperty(name="Material Name", default="", update=update_physics_props)

# -------------------------------------------------------------------
# 3. The UI Panel Class
# -------------------------------------------------------------------
class OBJECT_PT_CatalystPhysics(bpy.types.Panel):
    bl_label = "Catalyst Physics"
    bl_idname = "OBJECT_PT_catalyst_physics"
    bl_space_type = 'PROPERTIES'
    bl_region_type = 'WINDOW'
    bl_context = "object"

    def draw(self, context):
        layout = self.layout
        obj = context.object
        props = obj.catalyst_physics

        # Draw Main Dropdowns
        layout.prop(props, "cat_phys_body")
        layout.prop(props, "cat_phys_shape")

        # Separator
        layout.separator()

        # Logic to hide/show fields based on selection
        has_body = props.cat_phys_body != 'NONE'
        has_shape = props.cat_phys_shape != 'NONE'

        if has_body or has_shape:
            
            # DYNAMICS SECTION
            if props.cat_phys_body == 'dynamic':
                box = layout.box()
                box.label(text="Dynamics")
                box.prop(props, "cat_phys_mass")
                box.prop(props, "cat_phys_gravity_scale")
                
                row = box.row()
                row.prop(props, "cat_phys_linear_damping", text="Lin Damping")
                row.prop(props, "cat_phys_angular_damping", text="Ang Damping")

            # COLLISION SECTION
            box = layout.box()
            box.label(text="Collision Settings")
            box.prop(props, "cat_phys_is_trigger")
            
            row = box.row()
            row.prop(props, "cat_phys_layer", text="Layer")
            row.prop(props, "cat_phys_mask", text="Mask")
            
            box.prop(props, "cat_phys_material")

# -------------------------------------------------------------------
# 4. Registration
# -------------------------------------------------------------------
classes = (
    CatalystPhysicsSettings,
    OBJECT_PT_CatalystPhysics,
)

def register():
    for cls in classes:
        bpy.utils.register_class(cls)
    # Add the property group to all Objects
    bpy.types.Object.catalyst_physics = bpy.props.PointerProperty(type=CatalystPhysicsSettings)

def unregister():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
    del bpy.types.Object.catalyst_physics

if __name__ == "__main__":
    register()