#!/usr/bin/env python3
"""Real stdio MCP smoke test, including plugin tools and revision conflicts."""
import json,subprocess,sys
binary=sys.argv[1] if len(sys.argv)>1 else 'target/debug/erlu'
p=subprocess.Popen([binary,'mcp','--headless'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
seq=0
def rpc(method,params):
 global seq
 seq+=1
 p.stdin.write(json.dumps({'jsonrpc':'2.0','id':seq,'method':method,'params':params})+'\n');p.stdin.flush()
 while True:
  line=p.stdout.readline()
  if not line:raise RuntimeError(p.stderr.read())
  response=json.loads(line)
  if response.get('id')==seq:
   assert 'error' not in response,response
   return response['result']
def tool(name,args={}):
 return rpc('tools/call',{'name':name,'arguments':args})
try:
 rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'erlu-test','version':'1'}})
 p.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');p.stdin.flush()
 names={t['name'] for t in rpc('tools/list',{})['tools']}
 assert {'block_create','decompose','undo','export_pdf','plugin_colors_colors_set'}<=names,names
 first=tool('block_create',{'pos':{'x':40,'y':20},'title':'Test process','expected_revision':0})
 assert not first.get('isError'),first
 result=first['structuredContent'];new=result['result']['id'];revision=result['revision']
 stale=tool('objects_delete',{'ids':[new],'expected_revision':0})
 assert stale.get('isError'),stale
 assert 'revision_conflict' in str(stale)
 tool('selection_set',{'ids':[new]})
 colored=tool('plugin_colors_colors_set',{'color':'#ff0000','expected_revision':revision})
 assert not colored.get('isError'),colored
 state=tool('document_state')['structuredContent']
 assert state['result']['document']['diagrams']['1']['blocks'][str(new)]['color']=='#ff0000'
 tool('undo')
 restored=tool('object_get',{'id':new})['structuredContent']
 assert restored['result']['object']['color'] is None
 print(f'MCP stdio: {len(names)} tools, edits, conflicts, real WASM plugin, undo OK')
finally:
 p.terminate()
 try:p.wait(timeout=5)
 except subprocess.TimeoutExpired:p.kill()
