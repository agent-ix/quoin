import { canonicalJson } from "./src/evidence/store.js";
import { canonicalizeJcs } from "./src/change-assurance/integrity.js";
console.log(JSON.stringify(canonicalJson({ b: 1, a: [1, 2] })));
console.log(canonicalizeJcs({ b: 1, a: 1e21 }));
