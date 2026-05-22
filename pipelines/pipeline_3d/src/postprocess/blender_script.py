"""Blender headless script for mesh post-processing.

Invoked via: blender --background --python blender_script.py -- <input.glb> <output.glb> <max_vertices> <asset_type>
"""
import sys
import bpy
import bmesh
from pathlib import Path


def main():
    argv = sys.argv
    idx = argv.index("--") + 1
    input_path = argv[idx]
    output_path = argv[idx + 1]
    max_vertices = int(argv[idx + 2])
    asset_type = argv[idx + 3]

    bpy.ops.wm.read_factory_settings(use_empty=True)

    bpy.ops.import_scene.gltf(filepath=input_path)

    mesh_objects = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    if not mesh_objects:
        print("ERROR: No mesh objects found in file")
        sys.exit(1)

    for obj in mesh_objects:
        bpy.context.view_layer.objects.active = obj
        obj.select_set(True)

        bpy.ops.object.mode_set(mode="EDIT")
        bm = bmesh.from_edit_mesh(obj.data)

        total_verts = sum(len(o.data.vertices) for o in mesh_objects)
        if total_verts > max_vertices:
            ratio = max_vertices / total_verts
            target_faces = int(len(bm.faces) * ratio)
            target_faces = max(target_faces, 100)
            bpy.ops.object.mode_set(mode="OBJECT")
            modifier = obj.modifiers.new(name="Decimate", type="DECIMATE")
            modifier.ratio = ratio
            bpy.ops.object.modifier_apply(modifier=modifier.name)
            bpy.ops.object.mode_set(mode="EDIT")
            bm = bmesh.from_edit_mesh(obj.data)

        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        bmesh.update_edit_mesh(obj.data)
        bpy.ops.object.mode_set(mode="OBJECT")

        bbox = obj.bound_box
        center_x = sum(v[0] for v in bbox) / 8
        center_y = sum(v[1] for v in bbox) / 8
        min_z = min(v[2] for v in bbox)

        for vert in obj.data.vertices:
            vert.co.x -= center_x
            vert.co.y -= center_y
            vert.co.z -= min_z

        if not obj.data.uv_layers:
            bpy.ops.object.mode_set(mode="EDIT")
            bpy.ops.mesh.select_all(action="SELECT")
            bpy.ops.uv.smart_project(angle_limit=66.0, island_margin=0.02)
            bpy.ops.object.mode_set(mode="OBJECT")

        obj.select_set(False)

    for obj in mesh_objects:
        obj.select_set(True)

    bpy.ops.export_scene.gltf(
        filepath=output_path,
        export_format="GLB",
        use_selection=True,
        export_apply=True,
    )

    print(f"OK: Exported to {output_path}")


if __name__ == "__main__":
    main()
