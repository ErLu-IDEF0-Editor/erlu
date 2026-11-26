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
 assert 'plugin_responsibility_map_role_add' in names,names
 assert not tool('plugin_responsibility_map_role_add',{'name':'Operations'}).get('isError')
 for category in ['R','A']:
  assigned=tool('plugin_responsibility_map_assignment_set',{'role':'Operations','responsibility':category})
  assert not assigned.get('isError'),assigned
 matrix=tool('plugin_responsibility_map_matrix_export',{'scope':'selection','format':'csv'})
 assert not matrix.get('isError'),matrix
 assert 'R/A' in matrix['structuredContent']['result']['text'],matrix
 responsibility=tool('plugin_responsibility_map_validate',{'scope':'selection'})
 assert responsibility['structuredContent']['result']['valid'],responsibility
 assert 'plugin_semantic_dictionary_concept_add' in names,names
 assert not tool('plugin_semantic_dictionary_concept_add',{'code':'request','term':'Customer request','definition':'A customer request'}).get('isError')
 arrow=tool('arrow_create',{'from':{'anchor':{'kind':'boundary','port':'input','t':0.5}},'to':{'anchor':{'kind':'block','id':new,'port':'input','t':0.5}},'label':'req'})['structuredContent']['result']['id']
 tool('selection_set',{'ids':[arrow]})
 assert not tool('plugin_semantic_dictionary_binding_set',{'code':'request'}).get('isError')
 preview=tool('plugin_semantic_dictionary_normalize_preview',{'scope':'selection'})
 assert not preview.get('isError'),preview
 apply=preview['structuredContent']['result']['confirm_action']
 args={**apply['arguments'],'expected_revision':apply['expected_revision']}
 normalized=tool(apply['name'],args)
 assert not normalized.get('isError'),normalized
 assert tool('object_get',{'id':arrow})['structuredContent']['result']['object']['label']=='Customer request'
 assert {'dfd_new','dfd_edit'}<=names,names
 dfd=tool('dfd_new',{'title':'Fulfil order','discard':True})['structuredContent']
 assert tool('document_state')['structuredContent']['result']['document']['format']=='erlu-dfd'
 external=tool('dfd_edit',{'edit':{'operation':'node_create','kind':'external','title':'Customer','position':{'x':-70,'y':0}},'expected_revision':dfd['revision']})['structuredContent']
 incoming={'from':{'kind':'node','id':external['result']['id']},'to':{'kind':'node','id':2},'label':'Order','data':None}
 assert not tool('dfd_edit',{'edit':{'operation':'flow_create','flow':incoming}}).get('isError')
 issues=tool('validate')['structuredContent']['result']
 assert {d['code'] for d in issues}=={'dfd_missing_output','dfd_undocumented_flow'},issues
 assert 'wrong_notation' in str(tool('block_create',{'pos':{'x':0,'y':0}}))
 print(f'MCP stdio: {len(names)} tools, IDEF0 and DFD edits, conflicts, three WASM plugins and confirmed normalization OK')
finally:
 p.terminate()
 try:p.wait(timeout=5)
 except subprocess.TimeoutExpired:p.kill()
