# Code
- Prefer use before declare - i.e. write methods first, structs/enums/consts/types etc. later
- Doc comments only when writing methods or structs/enums/consts/types that seem ambiguous in their intent. Do not write regular comments
- Always destructure, except on types that implement Drop
- If we need to hand-roll our own solution, prefer that over pulling in dependencies