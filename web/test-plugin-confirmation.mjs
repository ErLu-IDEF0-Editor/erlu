import {test} from 'node:test';
import assert from 'node:assert/strict';
import {validatePluginConfirmation} from './runtime.js';
const manifest={id:'dictionary',actions:[{name:'preview',kind:'export'},{name:'apply',kind:'command'}]};
const good={name:'plugin_dictionary_apply',arguments:{roots:[2]},expected_revision:4};
test('confirmation only allows own registered write actions at the preview revision',()=>{
 assert.doesNotThrow(()=>validatePluginConfirmation(manifest,{confirm_action:good},4));
 assert.doesNotThrow(()=>validatePluginConfirmation(manifest,{},4));
 for(const changed of [null,{...good,name:'document_open'},{...good,name:'plugin_colors_colors_set'},
  {...good,name:'plugin_dictionary_preview'},{...good,expected_revision:3},{...good,arguments:[]},{...good,unexpected:true}]) {
  assert.throws(()=>validatePluginConfirmation(manifest,{confirm_action:changed},4));
 }
});
