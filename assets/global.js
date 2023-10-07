"use strict";(()=>{var be=Object.defineProperty;var we=(t,e)=>{for(var r in e)be(t,r,{get:e[r],enumerable:!0})};var i=(t,e)=>{if(!t)throw new Error(e??"Failed assertion.")},Y=t=>{throw new Error(t??"Reached unimplemented code.")},m=t=>{throw new Error(t??"Reached unreachable code.")};var H={};we(H,{base64:()=>se,hex:()=>D,json:()=>R,toml:()=>oe,utf8:()=>V,yaml:()=>ie});var Z=async t=>{try{return await syscall("build",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},ee=async t=>{try{return await syscall("bundle",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}};var te=async(t,e)=>{try{return await syscall("decompress",t,e)}catch(r){throw new Error("The syscall failed.",{cause:r})}},re=async(t,e)=>{try{return await syscall("download",t,e)}catch(r){throw new Error("The syscall failed.",{cause:r})}},k={base64:{decode:t=>{try{return syscall("encoding_base64_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_base64_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},hex:{decode:t=>{try{return syscall("encoding_hex_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_hex_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},json:{decode:t=>{try{return syscall("encoding_json_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_json_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},toml:{decode:t=>{try{return syscall("encoding_toml_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_toml_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},utf8:{decode:t=>{try{return syscall("encoding_utf8_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_utf8_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},yaml:{decode:t=>{try{return syscall("encoding_yaml_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_yaml_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}}},ne=async(t,e)=>{try{return await syscall("extract",t,e)}catch(r){throw new Error("The syscall failed.",{cause:r})}},ae=t=>{try{return syscall("log",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}};var G=async t=>{try{return await syscall("read",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}};var se;(r=>(r.decode=n=>k.base64.decode(n),r.encode=n=>k.base64.encode(n)))(se||={});var D;(r=>(r.decode=n=>k.hex.decode(n),r.encode=n=>k.hex.encode(n)))(D||={});var R;(r=>(r.decode=n=>k.json.decode(n),r.encode=n=>k.json.encode(n)))(R||={});var oe;(r=>(r.decode=n=>k.toml.decode(n),r.encode=n=>k.toml.encode(n)))(oe||={});var V;(r=>(r.decode=n=>k.utf8.decode(n),r.encode=n=>k.utf8.encode(n)))(V||={});var ie;(r=>(r.decode=n=>k.yaml.decode(n),r.encode=n=>k.yaml.encode(n)))(ie||={});var g;(e=>{class t{#e;constructor(n){this.#e=n}get state(){return this.#e}static withId(n){return new t({id:n,object:void 0})}static withObject(n){return new t({id:void 0,object:n})}expectId(){if(this.#e.id===void 0)throw new Error;return this.#e.id}expectObject(){if(this.#e.object===void 0)throw new Error;return this.#e.object}async id(){return await this.store(),this.#e.id}async object(){return await this.load(),this.#e.object}async load(){this.#e.object===void 0&&(this.#e.object=await syscall("load",this.#e.id))}async store(){this.#e.id===void 0&&(this.#e.id=await syscall("store",this.#e.object))}}e.Handle=t})(g||={});var O=t=>t.flat(1/0);var $=async(...t)=>await A.new(...t),A=class t{#e;constructor(e){this.#e=e}static async new(...e){let{contents:r,executable:n,references:a}=O(await Promise.all(e.map(async function o(l){let c=await b(l);return p.Arg.is(c)?{contents:c}:t.is(c)?{contents:await c.contents(),executable:await c.executable(),references:await c.references()}:c instanceof Array?await Promise.all(c.map(o)):typeof c=="object"?{contents:c.contents,executable:c.executable,references:c.references}:m()}))).reduce((o,{contents:l,executable:c,references:u})=>(o.contents.push(l),o.executable=c!==void 0?c:o.executable,o.references.push(...u??[]),o),{contents:[],executable:!1,references:[]}),s=await M(...r);return new t(g.Handle.withObject({kind:"file",value:{contents:s,executable:n,references:a}}))}static is(e){return e instanceof t}static expect(e){return i(t.is(e)),e}static assert(e){i(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return i(e.kind==="file"),e.value}get handle(){return this.#e}async contents(){return(await this.object()).contents}async executable(){return(await this.object()).executable}async references(){return(await this.object()).references}async size(){return(await this.contents()).size()}async bytes(){return(await this.contents()).bytes()}async text(){return(await this.contents()).text()}};var U=class t{#e;constructor(e){this.#e=e}static is(e){return e instanceof t}static expect(e){return i(t.is(e)),e}static assert(e){i(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return i(e.kind==="package"),e.value}get handle(){return this.#e}async artifact(){return(await this.object()).artifact}async dependencies(){return(await this.object()).dependencies}};var z=t=>j.new(t),j=class t{#e;constructor(e){this.#e=e}static new(e){return new t(e)}static is(e){return e instanceof t}get name(){return this.#e}};var _=(...t)=>v.new(...t),S=(...t)=>B.new(...t),v=class t{#e;#t;constructor(e){this.#e=e?.parents??0,this.#t=e?.subpath??new B}static new(...e){return e.reduce(function r(n,a){if(typeof a=="string")for(let s of a.split("/"))s===""||s==="."||(s===".."?n=n.parent():n.#t.push(s));else if(a instanceof t){for(let s=0;s<a.#e;s++)n.parent();n.#t.join(a.#t)}else if(a instanceof B)n.#t.join(a);else if(a instanceof Array)a.forEach(s=>r(n,s));else return m();return n},new t)}static is(e){return e instanceof t}isEmpty(){return this.#e==0&&this.#t.isEmpty()}parents(){return this.#e}subpath(){return this.#t}parent(){return this.#t.isEmpty()?this.#e+=1:this.#t.pop(),this}join(e){e=t.new(e);for(let r=0;r<e.#e;r++)this.parent();return this.#t.join(e.#t),this}extension(){return this.#t.extension()}toSubpath(){if(this.#e>0)throw new Error("Cannot convert to subpath.");return this.#t}toString(){let e="";for(let r=0;r<this.#e;r++)e+="../";return e+=this.#t.toString(),e}};(e=>{let t;(s=>(s.is=o=>o===void 0||typeof o=="string"||o instanceof B||o instanceof e||o instanceof Array&&o.every(e.Arg.is),s.expect=o=>(i((0,s.is)(o)),o),s.assert=o=>{i((0,s.is)(o))}))(t=e.Arg||={})})(v||={});var B=class t{#e;constructor(e){this.#e=e??[]}static new(...e){return v.new(...e).toSubpath()}static is(e){return e instanceof t}components(){return[...this.#e]}isEmpty(){return this.#e.length==0}join(e){return this.#e.push(...e.#e),this}push(e){this.#e.push(e)}pop(){this.#e.pop()}extension(){return this.#e.at(-1)?.split(".").at(-1)}toRelpath(){return v.new(this)}toString(){return this.#e.join("/")}};var C=async(t,...e)=>{let r=[];for(let n=0;n<t.length-1;n++){let a=t[n];r.push(a);let s=e[n];r.push(s)}return r.push(t[t.length-1]),await E(...r)},E=(...t)=>f.new(...t),f=class t{#e;constructor(e){this.#e=e}static async new(...e){let r=O(await Promise.all(e.map(async function n(a){return a=await b(a),t.Component.is(a)?a:a instanceof t?a.components:a instanceof Array?await Promise.all(a.map(n)):m()}))).reduce((n,a)=>(n.push(a),n),[]);return r=r.reduce((n,a)=>{let s=n.at(-1);return a===""||(typeof s=="string"&&typeof a=="string"?n.splice(-1,1,s+a):n.push(a)),n},[]),new t(r)}static is(e){return e instanceof t}static expect(e){return i(t.is(e)),e}static assert(e){i(t.is(e))}static async join(e,...r){let n=await E(e),a=await Promise.all(r.map(o=>E(o)));a=a.filter(o=>o.components.length>0);let s=[];for(let o=0;o<a.length;o++){o>0&&s.push(n);let l=a[o];i(l),s.push(l)}return E(...s)}get components(){return this.#e}};(e=>{let t;(n=>n.is=a=>typeof a=="string"||T.is(a)||a instanceof j)(t=e.Component||={})})(f||={});var q=async(...t)=>await w.new(...t),w=class t{#e;constructor(e){this.#e=e}static async new(...e){let{artifact:r,path:n}=O(await Promise.all(e.map(async function s(o){let l=await b(o);if(typeof l=="string")return{path:_(l)};if(T.is(l))return{artifact:l};if(l instanceof f){i(l.components.length<=2);let[c,u]=l.components;if(typeof c=="string"&&u===void 0)return{path:_(c)};if(T.is(c)&&u===void 0)return{artifact:c};if(T.is(c)&&typeof u=="string")return i(u.startsWith("/")),{artifact:c,path:_(u.slice(1))};throw new Error("Invalid template.")}else return l instanceof t?{artifact:await l.artifact(),path:await l.path()}:l instanceof Array?await Promise.all(l.map(s)):typeof l=="object"?{artifact:l.artifact,path:_(l.path)}:m()}))).reduce((s,{artifact:o,path:l})=>(o!==void 0?(s.artifact=o,s.path=l??_()):s.path=s.path.join(l),s),{artifact:void 0,path:_()}),a;if(r!==void 0&&!n.isEmpty())a=await C`${r}/${n.toString()}`;else if(r!==void 0)a=await C`${r}`;else if(!n.isEmpty())a=await C`${n.toString()}`;else throw new Error("Invalid symlink.");return new t(g.Handle.withObject({kind:"symlink",value:{target:a}}))}static is(e){return e instanceof t}static expect(e){return i(t.is(e)),e}static assert(e){i(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return i(e.kind==="symlink"),e.value}get handle(){return this.#e}async target(){return(await this.object()).target}async artifact(){let r=(await this.target()).components.at(0);if(T.is(r))return r}async path(){let e=await this.target(),[r,n]=e.components;if(typeof r=="string"&&n===void 0)return _(r);if(T.is(r)&&n===void 0)return _();if(T.is(r)&&typeof n=="string")return _(n.slice(1));throw new Error("Invalid template.")}async resolve(e){e=e?await q(e):void 0;let r=await e?.artifact();r instanceof t&&(r=await r.resolve());let n=e?.path(),a=await this.artifact();a instanceof t&&(a=await a.resolve());let s=await this.path();if(a!==void 0&&s.isEmpty())return a;if(a===void 0&&!s.isEmpty()){if(!(r instanceof y))throw new Error("Expected a directory.");return await r.tryGet((await(n??_())).parent().join(s).toSubpath().toString())}else if(a!==void 0&&!s.isEmpty()){if(!(a instanceof y))throw new Error("Expected a directory.");return await a.tryGet(s.toSubpath().toString())}else throw new Error("Invalid symlink.")}};var L={};function ce(...t){if(t.length===1&&typeof t[0]=="object"&&"function"in t[0]){let e=t[0],r=R.encode({module:{package:e.module.package.handle.expectId(),path:e.module.path},name:e.name});return i(L[r]===void 0),L[r]=e.function,P.new({host:"js-js",executable:e.module.path,package:e.module.package,name:e.name})}else return P.new(...t)}var le=async(...t)=>await(await P.new(...t)).build(),ue=z("output"),P=class t extends globalThis.Function{#e;constructor(e){super(),this.#e=e;let r=this;return new Proxy(r,{get(n,a,s){return typeof r[a]=="function"?r[a].bind(r):r[a]},apply:async(n,a,s)=>await(await t.new(r,{args:s})).build(),getPrototypeOf:n=>Object.getPrototypeOf(r)})}static async new(...e){let{host:r,executable:n,package_:a,name:s,env:o,args_:l,checksum:c,unsafe_:u}=O(await Promise.all(e.map(async function h(d){let x=await b(d);return x instanceof f?{}:x instanceof t?{}:x instanceof Array?await Promise.all(x.map(h)):typeof x=="object"?{host:x.host,executable:await E(x.executable),package_:x.package,name:s,env:o,args_:l,checksum:c,unsafe_:u}:m()}))).reduce((h,d)=>({host:h.host??d.host,executable:h.executable??d.executable,package_:h.package_??d.package_,name:h.name??d.name,env:{...h.env??{},...d.env??{}},args_:[...h.args_??[],...d.args_??[]],checksum:h.checksum??d.checksum,unsafe_:h.unsafe_??d.unsafe_}),{});if(!r)throw new Error("Cannot create a target without a host.");if(!n)throw new Error("Cannot create a target without an executable.");return o??={},l??=[],u??=!1,new t(g.Handle.withObject({kind:"target",value:{host:r,executable:n,package:a,name:s,env:o,args:l,checksum:c,unsafe:u}}))}static is(e){return e instanceof t}static expect(e){return i(t.is(e)),e}static assert(e){i(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return i(e.kind==="target"),e.value}get handle(){return this.#e}async host(){return(await this.object()).host}async executable(){return(await this.object()).executable}async package(){return(await this.object()).package}async name_(){return(await this.object()).name}async env(){return(await this.object()).env}async args(){return(await this.object()).args}async checksum(){return(await this.object()).checksum}async unsafe(){return(await this.object()).unsafe}async build(...e){return await Z(await t.new(this,{args:e}))}};var b=async t=>{if(t=await t,t===void 0||typeof t=="boolean"||typeof t=="number"||typeof t=="string"||t instanceof Uint8Array||t instanceof p||t instanceof y||t instanceof A||t instanceof w||t instanceof j||t instanceof f||t instanceof U||t instanceof P)return t;if(t instanceof Array)return await Promise.all(t.map(e=>b(e)));if(typeof t=="object")return Object.fromEntries(await Promise.all(Object.entries(t).map(async([e,r])=>[e,await b(r)])));throw new Error("Invalid value to resolve.")};var M=async(...t)=>await p.new(...t),de=async(t,e)=>await p.download(t,e),p=class t{#e;constructor(e){this.#e=e}static async new(...e){let r=O(await Promise.all(e.map(async function n(a){let s=await b(a);return s===void 0?[]:typeof s=="string"?new t(g.Handle.withObject({kind:"blob",value:V.encode(s)})):s instanceof Uint8Array?new t(g.Handle.withObject({kind:"blob",value:s})):s instanceof t?s:s instanceof Array?await Promise.all(s.map(n)):m()})));return r.length===0?new t(g.Handle.withObject({kind:"blob",value:new Uint8Array})):r.length===1?r[0]:new t(g.Handle.withObject({kind:"blob",value:await Promise.all(r.map(async n=>[n,await n.size()]))}))}static async download(e,r){return await re(e,r)}static is(e){return e instanceof t}static expect(e){return i(t.is(e)),e}static assert(e){i(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return i(e.kind==="blob"),e.value}get handle(){return this.#e}async size(){let e=await this.object();return e instanceof Array?e.map(([r,n])=>n).reduce((r,n)=>r+n,0):e.byteLength}async bytes(){return await G(this)}async text(){return V.decode(await G(this))}async decompress(e){return await te(this,e)}async extract(e){return await ne(this,e)}};(e=>{let t;(s=>(s.is=o=>o===void 0||typeof o=="string"||o instanceof Uint8Array||o instanceof e||o instanceof Array&&o.every(s.is),s.expect=o=>(i((0,s.is)(o)),o),s.assert=o=>{i((0,s.is)(o))}))(t=e.Arg||={})})(p||={});var me=async(...t)=>await y.new(...t),y=class t{#e;constructor(e){this.#e=e}static async new(...e){let r=await(await Promise.all(e.map(b))).reduce(async function n(a,s){let o=await a;if(s!==void 0)if(t.is(s))for(let[l,c]of Object.entries(await s.entries())){let u=o[l];t.is(u)&&t.is(c)&&(c=await t.new(u,c)),o[l]=c}else if(s instanceof Array)for(let l of s)o=await n(Promise.resolve(o),l);else if(typeof s=="object")for(let[l,c]of Object.entries(s)){let[u,...h]=S(l).components();if(u===void 0)throw new Error("The path must have at least one component.");let d=u,x=o[d];if(t.is(x)||(x=void 0),h.length>0){let I=S(h).toString(),ge=await t.new(x,{[I]:c});o[d]=ge}else if(c===void 0)delete o[d];else if(p.Arg.is(c)){let I=await $(c);o[d]=I}else if(A.is(c)||w.is(c))o[d]=c;else{let I=await t.new(x,c);o[d]=I}}else return m();return o},Promise.resolve({}));return new t(g.Handle.withObject({kind:"directory",value:{entries:r}}))}static is(e){return e instanceof t}static expect(e){return i(t.is(e)),e}static assert(e){i(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return i(e.kind==="directory"),e.value}get handle(){return this.#e}async get(e){let r=await this.tryGet(e);return i(r,`Failed to get the directory entry "${e}".`),r}async tryGet(e){let r=this,n=S();for(let a of S(e).components()){if(!t.is(r))return;n.push(a);let s=(await r.entries())[a];if(s===void 0)return;if(w.is(s)){let o=await s.resolve({artifact:this,path:n.toString()});if(o===void 0)return;r=o}else r=s}return r}async entries(){let e={};for await(let[r,n]of this)e[r]=n;return e}async bundle(){return await ee(this)}async*walk(){for await(let[e,r]of this)if(yield[S(e),r],t.is(r))for await(let[n,a]of r.walk())yield[S(e).join(n),a]}async*[Symbol.asyncIterator](){let e=await this.object();for(let[r,n]of Object.entries(e.entries))yield[r,n]}};var T;(n=>(n.is=a=>y.is(a)||A.is(a)||w.is(a),n.expect=a=>(i((0,n.is)(a)),a),n.assert=a=>{i((0,n.is)(a))}))(T||={});var K=class{message;location;stack;source;constructor(e,r,n,a){this.message=e,this.location=r,this.stack=n,this.source=a}},pe=(t,e)=>({callSites:e.map(n=>({typeName:n.getTypeName(),functionName:n.getFunctionName(),methodName:n.getMethodName(),fileName:n.getFileName(),lineNumber:n.getLineNumber(),columnNumber:n.getColumnNumber(),isEval:n.isEval(),isNative:n.isNative(),isConstructor:n.isConstructor(),isAsync:n.isAsync(),isPromiseAll:n.isPromiseAll(),promiseIndex:n.getPromiseIndex()}))});var ye=async t=>{let e=await t.module.package.artifact();y.assert(e);let r=S(t.module.path).toRelpath().parent().join(t.path).toSubpath().toString();return await e.get(r)};var J=(...t)=>{let e=t.map(r=>Ae(r)).join(" ");ae(e)},Ae=t=>W(t,new WeakSet),W=(t,e)=>{switch(typeof t){case"string":return`"${t}"`;case"number":return t.toString();case"boolean":return t?"true":"false";case"undefined":return"undefined";case"object":return t===null?"null":fe(t,e);case"function":return`(function "${t.name??"(anonymous)"}")`;case"symbol":return"(symbol)";case"bigint":return t.toString()}},fe=(t,e)=>{if(e.has(t))return"(circular)";if(e.add(t),t instanceof Array)return`[${t.map(r=>W(r,e)).join(", ")}]`;if(t instanceof Error)return t.message;if(t instanceof Promise)return"(promise)";if(t instanceof p)return`(tg.blob ${F(t.handle,e)})`;if(t instanceof y)return`(tg.directory ${F(t.handle,e)})`;if(t instanceof A)return`(tg.file ${F(t.handle,e)})`;if(t instanceof w)return`(tg.symlink ${F(t.handle,e)})`;if(t instanceof j)return`(tg.placeholder "${t.name}")`;if(t instanceof f)return`(tg.template "${t.components.map(n=>typeof n=="string"?n:`\${${W(n,e)}}`).join("")}")`;if(t instanceof U)return`(tg.package "${F(t.handle,e)}")`;if(t instanceof P)return`(tg.target "${F(t.handle,e)}")`;{let r="";t.constructor!==void 0&&t.constructor.name!=="Object"&&(r=`${t.constructor.name} `);let n=Object.entries(t).map(([a,s])=>`${a}: ${W(s,e)}`);return`${r}{ ${n.join(", ")} }`}},F=(t,e)=>{let{id:r,object:n}=t.state();return r!==void 0?r:n!==void 0?fe(n,e):m()};var he=t=>{if(typeof t=="string")return t;{let{arch:e,os:r}=t;return`${e}-${r}`}},Q;(n=>(n.is=a=>a==="aarch64-darwin"||a==="aarch64-linux"||a==="js-js"||a==="x86_64-darwin"||a==="x86_64-linux",n.arch=a=>{switch(a){case"aarch64-darwin":case"aarch64-linux":return"aarch64";case"js-js":return"js";case"x86_64-linux":case"x86_64-darwin":return"x86_64";default:throw new Error("Invalid system.")}},n.os=a=>{switch(a){case"aarch64-darwin":case"x86_64-darwin":return"darwin";case"js-js":return"js";case"x86_64-linux":case"aarch64-linux":return"linux";default:throw new Error("Invalid system.")}}))(Q||={});var X;(n=>(n.is=a=>a===void 0||typeof a=="boolean"||typeof a=="number"||typeof a=="string"||a instanceof Uint8Array||a instanceof p||a instanceof y||a instanceof A||a instanceof w||a instanceof j||a instanceof f||a instanceof U||a instanceof P||a instanceof Array||typeof a=="object",n.expect=a=>(i((0,n.is)(a)),a),n.assert=a=>{i((0,n.is)(a))}))(X||={});var xe=async t=>{let r=await(await t.package())?.id(),a=(await t.executable()).components[0],s={kind:"normal",value:{package:r,path:a}};await import(`tangram://${D.encode(V.encode(R.encode(s)))}/${a}`);let c=await t.name_();if(!c)throw new Error("The target must have a name.");let u=R.encode({module:{package:r,path:a},name:c}),h=L[u];if(!h)throw new Error("Failed to find the function.");let d=await t.args();return await h(...d)};Object.defineProperties(Error,{prepareStackTrace:{value:pe}});var ke={log:J};Object.defineProperties(globalThis,{console:{value:ke}});var je={Artifact:T,Blob:p,Directory:y,Error:K,File:A,Object_:g,Package:U,Placeholder:j,Symlink:w,System:Q,Target:P,Template:f,Value:X,assert:i,blob:M,build:le,directory:me,download:de,encoding:H,file:$,include:ye,log:J,main:xe,output:ue,placeholder:z,resolve:b,symlink:q,system:he,target:ce,template:E,unimplemented:Y,unreachable:m};Object.defineProperties(globalThis,{tg:{value:je},t:{value:C}});})();
//# sourceMappingURL=global.js.map
