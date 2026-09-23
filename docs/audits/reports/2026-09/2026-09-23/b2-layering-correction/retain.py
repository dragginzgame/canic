from pathlib import Path
import hashlib,json,shutil
repo=Path.cwd();base=repo/'.tmp/b2-layering-fix-20260923';reports=repo/'docs/audits/reports/2026-09/2026-09-23'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
prior=json.loads((reports/'b2-storage-closeout.json').read_text());correction=json.loads((base/'source-check.json').read_text())
overlay={r['path']:r['qualified_native_source_sha256'] for r in prior['native_only_overlay']};overlay[correction['path']]=correction['corrected_source_sha256']
manifest=json.loads((repo/'.tmp/b2-closeout-20260923/source-manifest.json').read_text())
for row in manifest:assert sha(repo/row['path'])==overlay.get(row['path'],row['after_sha256']),row['path']
assert 'test result: ok. 16 passed' in (base/'tests.log').read_text()
assert 'Finished ' in (base/'clippy.log').read_text() and 'error:' not in (base/'clippy.log').read_text()
assert (base/'layering.log').read_text()==''
out=reports/'b2-layering-correction';out.mkdir(exist_ok=True)
for name in ['source-check.json','native-test.patch','tests.log','clippy.log','layering.log','retain.py']:
 shutil.copyfile(base/name,out/name)
checks=[{'command':'bash scripts/ci/run-layering-guards.sh','result':'pass','log':'layering.log'},
 {'command':"RUSTC_WRAPPER='' CARGO_NET_OFFLINE=true cargo test --locked -p canic-control-plane --lib workflow::runtime::template",'result':'pass','log':'tests.log'},
 {'command':"RUSTC_WRAPPER='' CARGO_NET_OFFLINE=true cargo clippy --locked -p canic-control-plane --lib --tests --all-features -- -D warnings",'result':'pass','log':'clippy.log'}]
for check in checks:check['sha256']=sha(out/check['log'])
(out/'evidence.json').write_text(json.dumps({'schema_version':1,'date':'2026-09-23','scope':'Native B2 test setup corrected to use existing ops boundary','source':correction,'production_source_files_verified':len(manifest),'native_only_overlay_files':sorted(overlay),'canonical_evidence_unchanged':True,'previous_report_sha256':sha(reports/'b2-storage-closeout.json'),'checks':checks,'guard_changed':False,'artifacts_rebuilt':False,'broad_gate_run':False,'git_effects':False},indent=2)+'\n')
print('retained correction: 1994 source files verified with three native-only overlays; production inputs unchanged')
