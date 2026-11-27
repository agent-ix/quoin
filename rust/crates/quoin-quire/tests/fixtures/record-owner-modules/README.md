# Record key ownership module snapshots

These test inputs snapshot the manifests and schema files from the exact module
releases pinned by Quoin's `default-modules.yaml` at the PLAT-1965 baseline.
The ownership checks intentionally select only schemas whose root declares
`type: object` and a `properties` object; enum and helper schemas remain present
so `$ref` resolution follows the published module data.
The full root/property counts below guard the snapshot inventory. Ownership
assertions then select only those object schemas registered as active object
types with `data_schema`; nested property schemas such as `ReturningOperation`
and `PopulationMember` remain in the snapshot for `$ref` resolution but are not
record roots in their module manifests.

| Module | Source path | Release tag | Dereferenced commit | Object schemas | Properties |
| --- | --- | --- | --- | ---: | ---: |
| Architecture | `spec_objects_architecture/` | `v0.15.0` | `df01e2430759452fdea23ef95cd7e251976719c3` | 31 | 113 |
| Business | `spec_objects_business/` | `v0.11.0` | `8818fadbf6001dd1950cc2aa5703c874a90c3f16` | 24 | 76 |

The snapshot is test data, not a second authored module. Update it only when
the pinned release changes, and keep the tag and commit recorded here.
