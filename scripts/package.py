#!/usr/bin/env python3
"""Package one already-built native target; does not publish or sign artifacts."""
import argparse,pathlib,shutil,plistlib,subprocess,os
p=argparse.ArgumentParser();p.add_argument('--target',required=True);p.add_argument('--binary');p.add_argument('--appimage',action='store_true');a=p.parse_args()
root=pathlib.Path(__file__).resolve().parent.parent;out=root/'dist';out.mkdir(exist_ok=True)
windows='windows' in a.target;mac='apple' in a.target
target_root=pathlib.Path(os.environ.get('CARGO_TARGET_DIR',str(root/'target')))
if not target_root.is_absolute():target_root=root/target_root
binary=pathlib.Path(a.binary) if a.binary else target_root/a.target/'release'/('erlu.exe' if windows else 'erlu')
if not binary.exists():raise SystemExit(f'Missing binary: {binary}')
name=f'erlu-{a.target}';stage=out/name
if stage.exists():shutil.rmtree(stage)
if mac:
 app=stage/'erlu.app'/'Contents';dest=app/'MacOS';dest.mkdir(parents=True)
 (app/'Resources').mkdir();shutil.copy2(binary,dest/'erlu')
 (app/'Info.plist').write_bytes(plistlib.dumps({'CFBundleExecutable':'erlu','CFBundleIdentifier':'com.gregorybednov.erlu','CFBundleName':'erlu','CFBundleVersion':'0.1.0','CFBundleShortVersionString':'0.1.0','CFBundlePackageType':'APPL','NSHighResolutionCapable':True,'CFBundleDocumentTypes':[{'CFBundleTypeName':'erlu IDEF0','CFBundleTypeRole':'Editor','CFBundleTypeExtensions':['idef0']}] }))
else:
 dest=stage;dest.mkdir(parents=True);shutil.copy2(binary,dest/('erlu.exe' if windows else 'erlu'))
for plugin_id in ['colors','responsibility_map','semantic_dictionary']:
 plugins=dest/'plugins'/plugin_id;plugins.mkdir(parents=True)
 shutil.copy2(root/'plugins'/plugin_id/'plugin.json',plugins/'plugin.json')
 shutil.copytree(root/'plugins'/plugin_id/'dist',plugins/'dist')
shutil.copy2(root/'README.md',stage/'README.md');shutil.copytree(root/'assets/fonts',stage/'fonts')
if windows or mac:shutil.make_archive(str(out/name),'zip',stage.parent,stage.name)
else:shutil.make_archive(str(out/name),'gztar',stage.parent,stage.name)
if a.appimage:
 appdir=out/'erlu.AppDir'
 if appdir.exists():shutil.rmtree(appdir)
 bindir=appdir/'usr/bin';bindir.mkdir(parents=True)
 shutil.copy2(binary,bindir/'erlu');shutil.copytree(dest/'plugins',bindir/'plugins')
 desktop=appdir/'erlu.desktop';desktop.write_text('[Desktop Entry]\nType=Application\nName=erlu IDEF0 editor\nExec=erlu %f\nIcon=erlu\nCategories=Office;Graphics;\nMimeType=application/x-erlu-idef0;\n')
 shutil.copy2(root/'assets/icons/erlu.svg',appdir/'erlu.svg')
 # linuxdeploy bundles native libraries; exclude GPU drivers, which belong to the host.
 subprocess.run(['linuxdeploy','--appdir',str(appdir),'--executable',str(bindir/'erlu'),'--desktop-file',str(desktop),'--icon-file',str(appdir/'erlu.svg'),'--output','appimage'],check=True,cwd=out,env={**os.environ,'APPIMAGE_EXTRACT_AND_RUN':'1','LDAI_OUTPUT':f'erlu-{a.target}.AppImage'})
print(stage)
