"use strict";(()=>{var Pe=Object.defineProperty;var ve=(t,e)=>{for(var r in e)Pe(t,r,{get:e[r],enumerable:!0})};var o=(t,e)=>{if(!t)throw new Error(e??"Failed assertion.")},Y=t=>{throw new Error(t??"Reached unimplemented code.")},y=t=>{throw new Error(t??"Reached unreachable code.")};var B={};ve(B,{base64:()=>se,hex:()=>ie,json:()=>R,toml:()=>oe,utf8:()=>D,yaml:()=>le});var Z=async t=>{try{return await syscall("build",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},ee=async t=>{try{return await syscall("bundle",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}};var te=async(t,e)=>{try{return await syscall("decompress",t,e)}catch(r){throw new Error("The syscall failed.",{cause:r})}},re=async(t,e)=>{try{return await syscall("download",t,e)}catch(r){throw new Error("The syscall failed.",{cause:r})}},m={base64:{decode:t=>{try{return syscall("encoding_base64_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_base64_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},hex:{decode:t=>{try{return syscall("encoding_hex_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_hex_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},json:{decode:t=>{try{return syscall("encoding_json_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_json_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},toml:{decode:t=>{try{return syscall("encoding_toml_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_toml_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},utf8:{decode:t=>{try{return syscall("encoding_utf8_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_utf8_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},yaml:{decode:t=>{try{return syscall("encoding_yaml_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_yaml_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}}},ne=async(t,e)=>{try{return await syscall("extract",t,e)}catch(r){throw new Error("The syscall failed.",{cause:r})}},ae=t=>{try{return syscall("log",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}};var G=async t=>{try{return await syscall("read",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}};var se;(r=>(r.decode=n=>m.base64.decode(n),r.encode=n=>m.base64.encode(n)))(se||={});var ie;(r=>(r.decode=n=>m.hex.decode(n),r.encode=n=>m.hex.encode(n)))(ie||={});var R;(r=>(r.decode=n=>m.json.decode(n),r.encode=n=>m.json.encode(n)))(R||={});var oe;(r=>(r.decode=n=>m.toml.decode(n),r.encode=n=>m.toml.encode(n)))(oe||={});var D;(r=>(r.decode=n=>m.utf8.decode(n),r.encode=n=>m.utf8.encode(n)))(D||={});var le;(r=>(r.decode=n=>m.yaml.decode(n),r.encode=n=>m.yaml.encode(n)))(le||={});var p;(e=>{class t{#e;constructor(n){this.#e=n}get state(){return this.#e}static withId(n){return new t({id:n,object:void 0})}static withObject(n){return new t({id:void 0,object:n})}expectId(){if(this.#e.id===void 0)throw new Error;return this.#e.id}expectObject(){if(this.#e.object===void 0)throw new Error;return this.#e.object}async id(){return await this.store(),this.#e.id}async object(){return await this.load(),this.#e.object}async load(){this.#e.object===void 0&&(this.#e.object=await syscall("load",this.#e.id))}async store(){this.#e.id===void 0&&(this.#e.id=await syscall("store",this.#e.object))}}e.Handle=t})(p||={});var H=async(...t)=>await h.new(...t),ce=async(t,e)=>await h.download(t,e),h=class t{#e;constructor(e){this.#e=e}static withId(e){return new t(p.Handle.withId(e))}static async new(...e){let{children:r}=await j.apply(e,async n=>n===void 0?{children:[]}:typeof n=="string"?{children:{kind:"append",value:D.encode(n)}}:n instanceof Uint8Array?{children:{kind:"append",value:n}}:t.is(n)?{children:{kind:"append",value:await n.bytes()}}:y());return(!r||r.length===0)&&(r=[new Uint8Array]),new t(p.Handle.withObject({kind:"blob",value:await Promise.all(r.map(async n=>{let a=new t(p.Handle.withObject({kind:"blob",value:n}));return[a,await a.size()]}))}))}static async download(e,r){return await re(e,r)}static is(e){return e instanceof t}static expect(e){return o(t.is(e)),e}static assert(e){o(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return o(e.kind==="blob"),e.value}get handle(){return this.#e}async size(){let e=await this.object();return e instanceof Array?e.map(([r,n])=>n).reduce((r,n)=>r+n,0):e.byteLength}async bytes(){return await G(this)}async text(){return D.decode(await G(this))}async decompress(e){return await te(this,e)}async extract(e){return await ne(this,e)}};(e=>{let t;(i=>(i.is=s=>s===void 0||typeof s=="string"||s instanceof Uint8Array||e.is(s)||s instanceof Array&&s.every(i.is),i.expect=s=>(o((0,i.is)(s)),s),i.assert=s=>{o((0,i.is)(s))}))(t=e.Arg||={})})(h||={});var $=async(...t)=>await A.new(...t),A=class t{#e;constructor(e){this.#e=e}static withId(e){return new t(p.Handle.withId(e))}static async new(...e){let{contents:r,executable:n,references:a}=await j.apply(e,async l=>{if(h.Arg.is(l))return{contents:{kind:"append",value:l}};if(t.is(l)){let d={kind:"append",value:await l.contents()},k={kind:"append",value:await l.executable()},O={kind:"append",value:await l.references()};return{contents:d,executable:k,references:O}}else if(typeof l=="object"){let d={};return"contents"in l&&(d.contents={kind:"append",value:l.contents}),"executable"in l&&(d.executable={kind:"append",value:l.executable}),"references"in l&&(d.references={kind:"append",value:l.references}),d}else return y()}),i=await H(r),s=(n??[]).some(l=>l);return a??=[],new t(p.Handle.withObject({kind:"file",value:{contents:i,executable:s,references:a}}))}static is(e){return e instanceof t}static expect(e){return o(t.is(e)),e}static assert(e){o(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return o(e.kind==="file"),e.value}get handle(){return this.#e}async contents(){return(await this.object()).contents}async executable(){return(await this.object()).executable}async references(){return(await this.object()).references}async size(){return(await this.contents()).size()}async bytes(){return(await this.contents()).bytes()}async text(){return(await this.contents()).text()}};var F=(...t)=>U.new(...t),_=(...t)=>S.new(...t),U=class t{#e;#t;constructor(e){this.#e=e?.parents??0,this.#t=e?.subpath??new S}static new(...e){return e.reduce(function r(n,a){if(typeof a=="string")for(let i of a.split("/"))i===""||i==="."||(i===".."?n=n.parent():n.#t.push(i));else if(a instanceof t){for(let i=0;i<a.#e;i++)n.parent();n.#t.join(a.#t)}else if(a instanceof S)n.#t.join(a);else if(a instanceof Array)a.forEach(i=>r(n,i));else return y();return n},new t)}isEmpty(){return this.#e==0&&this.#t.isEmpty()}parents(){return this.#e}subpath(){return this.#t}parent(){return this.#t.isEmpty()?this.#e+=1:this.#t.pop(),this}join(e){e=t.new(e);for(let r=0;r<e.#e;r++)this.parent();return this.#t.join(e.#t),this}extension(){return this.#t.extension()}toSubpath(){if(this.#e>0)throw new Error("Cannot convert to subpath.");return this.#t}toString(){let e="";for(let r=0;r<this.#e;r++)e+="../";return e+=this.#t.toString(),e}};(e=>{let t;(i=>(i.is=s=>S.Arg.is(s)||s instanceof e||s instanceof Array&&s.every(e.Arg.is),i.expect=s=>(o((0,i.is)(s)),s),i.assert=s=>{o((0,i.is)(s))}))(t=e.Arg||={})})(U||={});var S=class{#e;constructor(e){this.#e=e??[]}static new(...e){return U.new(...e).toSubpath()}components(){return[...this.#e]}isEmpty(){return this.#e.length==0}join(e){return this.#e.push(...e.#e),this}push(e){this.#e.push(e)}pop(){this.#e.pop()}extension(){return this.#e.at(-1)?.split(".").at(-1)}toRelpath(){return U.new(this)}toString(){return this.#e.join("/")}};(e=>{let t;(i=>(i.is=s=>s===void 0||typeof s=="string"||s instanceof e||s instanceof Array&&s.every(e.Arg.is),i.expect=s=>(o((0,i.is)(s)),s),i.assert=s=>{o((0,i.is)(s))}))(t=e.Arg||={})})(S||={});var T;(n=>(n.is=a=>g.is(a)||A.is(a)||b.is(a),n.expect=a=>(o((0,n.is)(a)),a),n.assert=a=>{o((0,n.is)(a))}))(T||={});var w=(...t)=>u.new(...t),u=class t{#e;constructor(e){this.#e=e}static async new(...e){let{components:r}=await j.apply(e,async n=>n===void 0?{}:t.Component.is(n)?{components:{kind:"append",value:n}}:t.is(n)?{components:{kind:"append",value:n.components}}:y());return r=(r??[]).reduce((n,a)=>{let i=n.at(-1);return a===""||(typeof i=="string"&&typeof a=="string"?n.splice(-1,1,i+a):n.push(a)),n},[]),new t(r)}static is(e){return e instanceof t}static expect(e){return o(t.is(e)),e}static assert(e){o(t.is(e))}static async join(e,...r){let n=await w(e),a=await Promise.all(r.map(s=>w(s)));a=a.filter(s=>s.components.length>0);let i=[];for(let s=0;s<a.length;s++){s>0&&i.push(n);let l=a[s];o(l),i.push(l)}return w(...i)}get components(){return this.#e}};(r=>{let t;(a=>a.is=i=>i===void 0||e.is(i)||r.is(i)||i instanceof Array&&i.every(s=>a.is(s)))(t=r.Arg||={});let e;(a=>a.is=i=>typeof i=="string"||T.is(i))(e=r.Component||={})})(u||={});var W=async(...t)=>await b.new(...t),b=class t{#e;constructor(e){this.#e=e}static withId(e){return new t(p.Handle.withId(e))}static async new(...e){let{artifact:r,path:n}=await j.apply(e,async s=>{if(s==="undefined")return{artifact:{kind:"unset"}};if(typeof s=="string")return{path:{kind:"append",value:s}};if(T.is(s))return{artifact:{kind:"set",value:s}};if(u.is(s)){o(s.components.length<=2);let[l,d]=s.components;if(typeof l=="string"&&d===void 0)return{path:{kind:"set",value:[l]}};if(T.is(l)&&d===void 0)return{artifact:{kind:"set",value:l}};if(T.is(l)&&typeof d=="string")return o(d.startsWith("/")),{artifact:{kind:"set",value:l},path:{kind:"set",value:[d.slice(1)]}};throw new Error("Invalid template.")}else return t.is(s)?{artifact:{kind:"set",value:await s.artifact()},path:{kind:"set",value:[(await s.path()).toString()]}}:typeof s=="object"?{artifact:{kind:"set",value:s.artifact},path:{kind:"set",value:s.path?[s.path]:[]}}:y()}),a=F(...n??[]),i;if(r!==void 0&&!a.isEmpty())i=await w(r,"/",a.toString());else if(r!==void 0)i=await w(r);else if(!a.isEmpty())i=await w(a.toString());else throw new Error("Invalid symlink.");return new t(p.Handle.withObject({kind:"symlink",value:{target:i}}))}static is(e){return e instanceof t}static expect(e){return o(t.is(e)),e}static assert(e){o(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return o(e.kind==="symlink"),e.value}get handle(){return this.#e}async target(){return(await this.object()).target}async artifact(){let r=(await this.target()).components.at(0);if(T.is(r))return r}async path(){let e=await this.target(),[r,n]=e.components;if(typeof r=="string"&&n===void 0)return F(r);if(T.is(r)&&n===void 0)return F();if(T.is(r)&&typeof n=="string")return F(n.slice(1));throw new Error("Invalid template.")}async resolve(e){e=e?await W(e):void 0;let r=await e?.artifact();t.is(r)&&(r=await r.resolve());let n=e?.path(),a=await this.artifact();t.is(a)&&(a=await a.resolve());let i=await this.path();if(a!==void 0&&i.isEmpty())return a;if(a===void 0&&!i.isEmpty()){if(!g.is(r))throw new Error("Expected a directory.");return await r.tryGet((await(n??F())).parent().join(i).toSubpath().toString())}else if(a!==void 0&&!i.isEmpty()){if(!g.is(a))throw new Error("Expected a directory.");return await a.tryGet(i.toSubpath().toString())}else throw new Error("Invalid symlink.")}};var de=async(...t)=>await g.new(...t),g=class t{#e;constructor(e){this.#e=e}static withId(e){return new t(p.Handle.withId(e))}static async new(...e){let r=await(await Promise.all(e.map(E))).reduce(async function n(a,i){let s=await a;if(i!==void 0)if(t.is(i))for(let[l,d]of Object.entries(await i.entries())){let k=s[l];t.is(k)&&t.is(d)&&(d=await t.new(k,d)),s[l]=d}else if(i instanceof Array)for(let l of i)s=await n(Promise.resolve(s),l);else if(typeof i=="object")for(let[l,d]of Object.entries(i)){let[k,...O]=_(l).components();if(k===void 0)throw new Error("The path must have at least one component.");let c=k,f=s[c];if(t.is(f)||(f=void 0),O.length>0){let v=_(O).toString(),M=await t.new(f,{[v]:d});s[c]=M}else if(d===void 0)delete s[c];else if(h.Arg.is(d)){let v=await $(d);s[c]=v}else if(A.is(d)||b.is(d))s[c]=d;else{let v=await t.new(f,d);s[c]=v}}else return y();return s},Promise.resolve({}));return new t(p.Handle.withObject({kind:"directory",value:{entries:r}}))}static is(e){return e instanceof t}static expect(e){return o(t.is(e)),e}static assert(e){o(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return o(e.kind==="directory"),e.value}get handle(){return this.#e}async get(e){let r=await this.tryGet(e);return o(r,`Failed to get the directory entry "${e}".`),r}async tryGet(e){let r=this,n=_();for(let a of _(e).components()){if(!t.is(r))return;n.push(a);let i=(await r.entries())[a];if(i===void 0)return;if(b.is(i)){let s=await i.resolve({artifact:this,path:n.toString()});if(s===void 0)return;r=s}else r=i}return r}async entries(){let e={};for await(let[r,n]of this)e[r]=n;return e}async bundle(){return await ee(this)}async*walk(){for await(let[e,r]of this)if(yield[_(e),r],t.is(r))for await(let[n,a]of r.walk())yield[_(e).join(n),a]}async*[Symbol.asyncIterator](){let e=await this.object();for(let[r,n]of Object.entries(e.entries))yield[r,n]}};var x=class t{#e;constructor(e){this.#e=e}static withId(e){return new t(p.Handle.withId(e))}static is(e){return e instanceof t}static expect(e){return o(t.is(e)),e}static assert(e){o(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return o(e.kind==="package"),e.value}get handle(){return this.#e}async artifact(){return(await this.object()).artifact}async dependencies(){return(await this.object()).dependencies}};var I;(r=>(r.toUrl=n=>`tangram://${m.hex.encode(m.utf8.encode(m.json.encode(n)))}/${n.value.path}`,r.fromUrl=n=>{let a=n.match(/^tangram:\/\/(.*)\/(.*)$/);o(a);let[i,s,l]=a;return o(s!==void 0),m.json.decode(m.utf8.decode(m.hex.decode(s)))}))(I||={});var ue,pe=t=>{ue=t},z={};function q(...t){if(t.length===1&&typeof t[0]=="object"&&"function"in t[0]){let e=t[0],{url:r,name:n}=e,a=R.encode({url:r,name:n});o(z[a]===void 0),z[a]=e.function;let i=I.fromUrl(e.url);o(i.kind==="normal");let s=x.withId(i.value.packageId);return new P(p.Handle.withObject({kind:"target",value:{host:"js-js",executable:new u([i.value.path]),package:s,name:e.name,args:[],env:{},checksum:void 0,unsafe:!1}}))}else return P.new(...t)}var me=async(...t)=>await(await q(...t)).build(),P=class t extends globalThis.Function{#e;constructor(e){super(),this.#e=e;let r=this;return new Proxy(r,{get(n,a,i){return typeof r[a]=="function"?r[a].bind(r):r[a]},apply:async(n,a,i)=>await(await t.new(r,{args:i})).build(),getPrototypeOf:n=>Object.getPrototypeOf(r)})}static withId(e){return new t(p.Handle.withId(e))}static async new(...e){let{host:r,executable:n,package:a,name:i,env:s,args:l,checksum:d,unsafe:k}=await j.apply(e,async c=>{if(u.Arg.is(c)){let f={kind:"set",value:(await ue.env()).TANGRAM_HOST},v={kind:"set",value:await w("/bin/sh")},M={kind:"set",value:["-c",await w(c)]};return{host:f,executable:v,args_:M}}else if(t.is(c)){let f={kind:"set",value:await c.host()},v={kind:"set",value:await c.executable()},M={kind:"set",value:await c.package()},Ae={kind:"set",value:await c.name_()},xe={kind:"set",value:await c.env()},ke={kind:"set",value:await c.args()},je={kind:"set",value:await c.checksum()},Te={kind:"set",value:await c.unsafe()};return{host:f,executable:v,package:M,name:Ae,env:xe,args_:ke,checksum:je,unsafe:Te}}else if(typeof c=="object"){let f={};return"host"in c&&(f.host={kind:"set",value:c.host}),"executable"in c&&(f.executable={kind:"set",value:await w(c.executable)}),"package"in c&&(f.package={kind:"set",value:c.package}),"name"in c&&(f.name={kind:"set",value:c.name}),"env"in c&&(f.env=c.env===void 0?{kind:"unset"}:{kind:"append",value:c.env}),"args"in c&&(f.args={kind:"append",value:c.args}),"checksum"in c&&(f.checksum={kind:"set",value:c.checksum}),"unsafe"in c&&(f.unsafe={kind:"set",value:c.unsafe}),f}else return y()});if(!r)throw new Error("Cannot create a target without a host.");if(!n)throw new Error("Cannot create a target without an executable.");let O=s&&s instanceof Array?await _e({},...s??[]):s??{};return l??=[],k??=!1,new t(p.Handle.withObject({kind:"target",value:{host:r,executable:n,package:a,name:i,env:O,args:l,checksum:d,unsafe:k}}))}static is(e){return e instanceof t}static expect(e){return o(t.is(e)),e}static assert(e){o(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return o(e.kind==="target"),e.value}get handle(){return this.#e}async host(){return(await this.object()).host}async executable(){return(await this.object()).executable}async package(){return(await this.object()).package}async name_(){return(await this.object()).name}async env(){return(await this.object()).env}async args(){return(await this.object()).args}async checksum(){return(await this.object()).checksum}async unsafe(){return(await this.object()).unsafe}async build(...e){return await Z(await t.new(this,{args:e}))}},_e=async(t,...e)=>{let r={...t};for(let n of e)for(let[a,i]of Object.entries(n))await Oe(r,a,i);return r},Oe=async(t,e,r)=>{if(u.Arg.is(r)&&(r={kind:"set",value:r}),r.kind==="unset")delete t[e];else if(r.kind==="set")t[e]=r.value;else if(r.kind==="set_if_unset")e in t||(t[e]=r.value);else if(r.kind==="append"){e in t||(t[e]=await w());let n=t[e];o(u.Arg.is(n)),t[e]=await u.join(r.separator??"",n,...V(r.value))}else if(r.kind==="prepend"){e in t||(t[e]=await w());let n=t[e];o(u.Arg.is(n)),t[e]=await u.join(r.separator??"",...V(r.value),n)}};var E=async t=>{if(t=await t,t===void 0||typeof t=="boolean"||typeof t=="number"||typeof t=="string"||t instanceof Uint8Array||t instanceof h||t instanceof g||t instanceof A||t instanceof b||t instanceof u||t instanceof x||t instanceof P)return t;if(t instanceof Array)return await Promise.all(t.map(e=>E(e)));if(typeof t=="object")return Object.fromEntries(await Promise.all(Object.entries(t).map(async([e,r])=>[e,await E(r)])));throw new Error("Invalid value to resolve.")};var j;(e=>e.apply=async(r,n)=>V(await Promise.all(V(await Promise.all(r.map(E))).map(a=>n(a)))).reduce((a,i)=>{for(let[s,l]of Object.entries(i))Se(a,s,l);return a},{}))(j||={});var V=t=>t instanceof Array?t.flat(1/0):[t],Se=(t,e,r)=>{if(r.kind==="unset")delete t[e];else if(r.kind==="set")t[e]=r.value;else if(r.kind==="set_if_unset")e in t||(t[e]=r.value);else if(r.kind==="prepend"){e in t||(t[e]=[]);let n=t[e];o(n instanceof Array),n.unshift(...V(r.value))}else if(r.kind==="append"){e in t||(t[e]=[]);let n=t[e];o(n instanceof Array),n.push(...V(r.value))}};var L=class{message;location;stack;source;constructor(e,r,n,a){this.message=e,this.location=r,this.stack=n,this.source=a}},fe=(t,e)=>({callSites:e.map(n=>({typeName:n.getTypeName(),functionName:n.getFunctionName(),methodName:n.getMethodName(),fileName:n.getFileName(),lineNumber:n.getLineNumber(),columnNumber:n.getColumnNumber(),isEval:n.isEval(),isNative:n.isNative(),isConstructor:n.isConstructor(),isAsync:n.isAsync(),isPromiseAll:n.isPromiseAll(),promiseIndex:n.getPromiseIndex()}))});var ye=async t=>{let e=I.fromUrl(t.url);o(e.kind==="normal");let n=await x.withId(e.value.packageId).artifact();g.assert(n);let a=_(e.value.path).toRelpath().parent().join(t.path).toSubpath().toString();return await n.get(a)};var J=(...t)=>{let e=t.map(r=>Ee(r)).join(" ");ae(e)},Ee=t=>K(t,new WeakSet),K=(t,e)=>{switch(typeof t){case"string":return`"${t}"`;case"number":return t.toString();case"boolean":return t?"true":"false";case"undefined":return"undefined";case"object":return t===null?"null":he(t,e);case"function":return`(function "${t.name??"(anonymous)"}")`;case"symbol":return"(symbol)";case"bigint":return t.toString()}},he=(t,e)=>{if(e.has(t))return"(circular)";if(e.add(t),t instanceof Array)return`[${t.map(r=>K(r,e)).join(", ")}]`;if(t instanceof Error)return t.message;if(t instanceof Promise)return"(promise)";if(h.is(t))return`(tg.blob ${C(t.handle,e)})`;if(g.is(t))return`(tg.directory ${C(t.handle,e)})`;if(A.is(t))return`(tg.file ${C(t.handle,e)})`;if(b.is(t))return`(tg.symlink ${C(t.handle,e)})`;if(u.is(t))return`(tg.template "${t.components.map(n=>typeof n=="string"?n:`\${${K(n,e)}}`).join("")}")`;if(x.is(t))return`(tg.package "${C(t.handle,e)}")`;if(P.is(t))return`(tg.target "${C(t.handle,e)}")`;{let r="";t.constructor!==void 0&&t.constructor.name!=="Object"&&(r=`${t.constructor.name} `);let n=Object.entries(t).map(([a,i])=>`${a}: ${K(i,e)}`);return`${r}{ ${n.join(", ")} }`}},C=(t,e)=>{let{id:r,object:n}=t.state;return r!==void 0?r:n!==void 0?he(n,e):y()};var ge=async t=>{let e=await t.package();o(e);let r=await e.id(),a=(await t.executable()).components[0];o(typeof a=="string");let i={kind:"normal",value:{packageId:r,path:a}},s=I.toUrl(i);await import(s);let l=await t.name_();if(!l)throw new Error("The target must have a name.");let d=R.encode({url:s,name:l}),k=z[d];if(!k)throw new Error("Failed to find the function.");pe(t);let O=await t.args();return await k(...O)};var be=t=>{if(typeof t=="string")return t;{let{arch:e,os:r}=t;return`${e}-${r}`}},Q;(n=>(n.is=a=>a==="aarch64-darwin"||a==="aarch64-linux"||a==="js-js"||a==="x86_64-darwin"||a==="x86_64-linux",n.arch=a=>{switch(a){case"aarch64-darwin":case"aarch64-linux":return"aarch64";case"js-js":return"js";case"x86_64-linux":case"x86_64-darwin":return"x86_64";default:throw new Error("Invalid system.")}},n.os=a=>{switch(a){case"aarch64-darwin":case"x86_64-darwin":return"darwin";case"js-js":return"js";case"x86_64-linux":case"aarch64-linux":return"linux";default:throw new Error("Invalid system.")}}))(Q||={});var X;(n=>(n.is=a=>a===void 0||typeof a=="boolean"||typeof a=="number"||typeof a=="string"||a instanceof Uint8Array||a instanceof h||a instanceof g||a instanceof A||a instanceof b||a instanceof u||a instanceof x||a instanceof P||a instanceof Array||typeof a=="object",n.expect=a=>(o((0,n.is)(a)),a),n.assert=a=>{o((0,n.is)(a))}))(X||={});Object.defineProperties(Error,{prepareStackTrace:{value:fe}});var Ue={log:J};Object.defineProperties(globalThis,{console:{value:Ue}});async function we(t,...e){let r=[];for(let n=0;n<t.length-1;n++){let a=t[n];r.push(a);let i=e[n];r.push(i)}return r.push(t[t.length-1]),await w(...r)}Object.assign(we,{Args:j,Artifact:T,Blob:h,Directory:g,Error:L,File:A,Object_:p,Package:x,Symlink:b,System:Q,Target:P,Template:u,Value:X,assert:o,blob:H,build:me,directory:de,download:ce,encoding:B,file:$,include:ye,log:J,main:ge,resolve:E,symlink:W,system:be,target:q,template:w,unimplemented:Y,unreachable:y});Object.defineProperties(globalThis,{tg:{value:we}});})();
//# sourceMappingURL=runtime.js.map
