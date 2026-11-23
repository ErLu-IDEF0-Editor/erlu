import {execFileSync} from 'node:child_process';
import {mkdirSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
const root=fileURLToPath(new URL('../',import.meta.url));
const targetDir=resolve(root,process.env.CARGO_TARGET_DIR??'target/plugins');
const env={...process.env,CARGO_TARGET_DIR:targetDir,CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:'-C link-arg=--max-memory=33554432'};
for(const [id,artifact] of [['colors','erlu_colors'],['responsibility_map','erlu_responsibility_map'],['semantic_dictionary','erlu_semantic_dictionary']]) {
  const directory=resolve(root,'plugins',id);
  mkdirSync(`${directory}/dist`,{recursive:true});
  execFileSync('cargo',['build','--manifest-path',`${directory}/Cargo.toml`,'--target','wasm32-unknown-unknown','--release','--locked'],{cwd:root,stdio:'inherit',env});
  execFileSync('wasm-tools',['component','new',`${targetDir}/wasm32-unknown-unknown/release/${artifact}.wasm`,'-o',`${directory}/dist/${id}.wasm`],{stdio:'inherit'});
  execFileSync(process.execPath,[fileURLToPath(new URL('node_modules/@bytecodealliance/jco/src/jco.js',import.meta.url)),'transpile',`${directory}/dist/${id}.wasm`,'--name',id,'-o',`${directory}/dist`,'--no-typescript'],{stdio:'inherit'});
}
