#!/usr/bin/env node

import {
  DEFAULT_ROOT,
  auditProductCapabilityRegistry,
  buildCapabilityView,
  loadProductCapabilityRegistry,
  writeProductCapabilityArtifacts,
} from './product_capability.mjs';

/** @GOV
 * codes: BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-001, BR-PCG-STATE-001-RB-PCG-PCG-GENERATE-001
 * type: RB
 * chain: PCG-GENERATE
 * rules: BR-PCG-DATA-001, BR-PCG-STATE-001
 * boundary: in=ProductCapability registry | out=generated ProductCapabilityAtlas artifacts | effect=WS-product-capability-atlas | does_not=mutate_product_runtime_or_registry
 * term_ref: TERM-PCG-001, TERM-PCG-002, TERM-PCG-003
 */
function main() {
  const loaded = loadProductCapabilityRegistry(DEFAULT_ROOT);
  const audit = auditProductCapabilityRegistry(DEFAULT_ROOT, loaded.registry);
  if (audit.status !== 'passed') {
    console.error(JSON.stringify(audit, null, 2));
    process.exitCode = 1;
    return;
  }
  const view = buildCapabilityView(loaded.registry);
  writeProductCapabilityArtifacts(DEFAULT_ROOT, loaded.config, view, audit);
  process.stdout.write(`${JSON.stringify({ status: 'passed', output_dir: loaded.config.product_capability_output_dir, summary: audit.summary }, null, 2)}\n`);
}

main();
