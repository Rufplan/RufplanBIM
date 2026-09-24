"""Validates an IFC file with IfcOpenShell: schema rules plus the counts the sample expects.

Usage: python tools/validate_ifc.py target/sample-model.ifc
"""

import sys

import ifcopenshell
import ifcopenshell.validate

EXPECTED = {
    "IfcBuildingStorey": 3,
    "IfcWall": 11,
    "IfcDoor": 3,
    "IfcWindow": 12,
    "IfcOpeningElement": 15,
    "IfcSlab": 2,
    "IfcCovering": 3,
    "IfcSpace": 5,
    "IfcRoof": 1,
    "IfcStair": 1,
    "IfcMaterialLayerSet": 2,
}


def main(path: str) -> int:
    model = ifcopenshell.open(path)
    if model.schema != "IFC4":
        print(f"expected IFC4, got {model.schema}")
        return 1

    logger = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(model, logger, express_rules=True)
    problems = logger.statements
    for p in problems:
        print(f"{p.get('level', '?')}: {p.get('message')} ({p.get('instance')})")

    failures = 0
    for entity, count in EXPECTED.items():
        got = len(model.by_type(entity))
        if got != count:
            print(f"{entity}: expected {count}, got {got}")
            failures += 1

    guids = [e.GlobalId for e in model.by_type("IfcRoot")]
    if len(guids) != len(set(guids)):
        print("duplicate GlobalIds")
        failures += 1

    for wall in model.by_type("IfcWall"):
        if wall.ContainedInStructure == ():
            print(f"{wall.GlobalId} is not contained in a storey")
            failures += 1

    print(f"{len(problems)} schema problem(s), {failures} check failure(s)")
    return 1 if problems or failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
