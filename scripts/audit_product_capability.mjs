#!/usr/bin/env node

import { DEFAULT_ROOT, auditProductCapabilityRegistry } from './product_capability.mjs';

/** @GOV
 * codes: BR-PCG-VERIFY-001-RB-PCG-PCG-AUDIT-001, BR-PCG-VERIFY-002-RB-PCG-PCG-AUDIT-001
 * type: RB
 * chain: PCG-AUDIT
 * rules: BR-PCG-VERIFY-001, BR-PCG-VERIFY-002
 * boundary: in=ProductCapability registry and authority references | out=traceability and CanonicalChain audit result | does_not=write_registry_or_change_product_code
 * term_ref: TERM-PCG-001, TERM-PCG-002, TERM-PCG-004
 */
const result = auditProductCapabilityRegistry(DEFAULT_ROOT);
process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
if (result.status !== 'passed') process.exitCode = 1;
