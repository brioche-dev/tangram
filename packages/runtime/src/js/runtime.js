"use strict";(()=>{var we=Object.defineProperty;var Ae=(t,e)=>{for(var r in e)we(t,r,{get:e[r],enumerable:!0})};var o=(t,e)=>{if(!t)throw new Error(e??"Failed assertion.")},X=t=>{throw new Error(t??"Reached unimplemented code.")},f=t=>{throw new Error(t??"Reached unreachable code.")};var V={};Ae(V,{base64:()=>ae,hex:()=>se,json:()=>R,toml:()=>ie,utf8:()=>H,yaml:()=>oe});var Y=async t=>{try{return await syscall("build",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},Z=async t=>{try{return await syscall("bundle",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}};var ee=async(t,e)=>{try{return await syscall("decompress",t,e)}catch(r){throw new Error("The syscall failed.",{cause:r})}},te=async(t,e)=>{try{return await syscall("download",t,e)}catch(r){throw new Error("The syscall failed.",{cause:r})}},m={base64:{decode:t=>{try{return syscall("encoding_base64_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_base64_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},hex:{decode:t=>{try{return syscall("encoding_hex_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_hex_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},json:{decode:t=>{try{return syscall("encoding_json_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_json_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},toml:{decode:t=>{try{return syscall("encoding_toml_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_toml_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},utf8:{decode:t=>{try{return syscall("encoding_utf8_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_utf8_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},yaml:{decode:t=>{try{return syscall("encoding_yaml_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("encoding_yaml_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}}},re=async(t,e)=>{try{return await syscall("extract",t,e)}catch(r){throw new Error("The syscall failed.",{cause:r})}},ne=t=>{try{return syscall("log",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}};var K=async t=>{try{return await syscall("read",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}};var ae;(r=>(r.decode=n=>m.base64.decode(n),r.encode=n=>m.base64.encode(n)))(ae||={});var se;(r=>(r.decode=n=>m.hex.decode(n),r.encode=n=>m.hex.encode(n)))(se||={});var R;(r=>(r.decode=n=>m.json.decode(n),r.encode=n=>m.json.encode(n)))(R||={});var ie;(r=>(r.decode=n=>m.toml.decode(n),r.encode=n=>m.toml.encode(n)))(ie||={});var H;(r=>(r.decode=n=>m.utf8.decode(n),r.encode=n=>m.utf8.encode(n)))(H||={});var oe;(r=>(r.decode=n=>m.yaml.decode(n),r.encode=n=>m.yaml.encode(n)))(oe||={});var d;(e=>{class t{#e;constructor(n){this.#e=n}get state(){return this.#e}static withId(n){return new t({id:n,object:void 0})}static withObject(n){return new t({id:void 0,object:n})}expectId(){if(this.#e.id===void 0)throw new Error;return this.#e.id}expectObject(){if(this.#e.object===void 0)throw new Error;return this.#e.object}async id(){return await this.store(),this.#e.id}async object(){return await this.load(),this.#e.object}async load(){this.#e.object===void 0&&(this.#e.object=await syscall("load",this.#e.id))}async store(){this.#e.id===void 0&&(this.#e.id=await syscall("store",this.#e.object))}}e.Handle=t})(d||={});var O=t=>t.flat(1/0);var D=async(...t)=>await k.new(...t),k=class t{#e;constructor(e){this.#e=e}static withId(e){return new t(d.Handle.withId(e))}static async new(...e){let{contents:r,executable:n,references:a}=O(await Promise.all(e.map(async function i(l){let c=await w(l);return h.Arg.is(c)?{contents:c}:t.is(c)?{contents:await c.contents(),executable:await c.executable(),references:await c.references()}:c instanceof Array?await Promise.all(c.map(i)):typeof c=="object"?{contents:c.contents,executable:c.executable,references:c.references}:f()}))).reduce((i,{contents:l,executable:c,references:p})=>(i.contents.push(l),i.executable=c!==void 0?c:i.executable,i.references.push(...p??[]),i),{contents:[],executable:!1,references:[]}),s=await M(...r);return new t(d.Handle.withObject({kind:"file",value:{contents:s,executable:n,references:a}}))}static is(e){return e instanceof t}static expect(e){return o(t.is(e)),e}static assert(e){o(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return o(e.kind==="file"),e.value}get handle(){return this.#e}async contents(){return(await this.object()).contents}async executable(){return(await this.object()).executable}async references(){return(await this.object()).references}async size(){return(await this.contents()).size()}async bytes(){return(await this.contents()).bytes()}async text(){return(await this.contents()).text()}};var j=class t{#e;constructor(e){this.#e=e}static withId(e){return new t(d.Handle.withId(e))}static is(e){return e instanceof t}static expect(e){return o(t.is(e)),e}static assert(e){o(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return o(e.kind==="package"),e.value}get handle(){return this.#e}async artifact(){return(await this.object()).artifact}async dependencies(){return(await this.object()).dependencies}};var _=(...t)=>v.new(...t),S=(...t)=>E.new(...t),v=class t{#e;#t;constructor(e){this.#e=e?.parents??0,this.#t=e?.subpath??new E}static new(...e){return e.reduce(function r(n,a){if(typeof a=="string")for(let s of a.split("/"))s===""||s==="."||(s===".."?n=n.parent():n.#t.push(s));else if(a instanceof t){for(let s=0;s<a.#e;s++)n.parent();n.#t.join(a.#t)}else if(a instanceof E)n.#t.join(a);else if(a instanceof Array)a.forEach(s=>r(n,s));else return f();return n},new t)}isEmpty(){return this.#e==0&&this.#t.isEmpty()}parents(){return this.#e}subpath(){return this.#t}parent(){return this.#t.isEmpty()?this.#e+=1:this.#t.pop(),this}join(e){e=t.new(e);for(let r=0;r<e.#e;r++)this.parent();return this.#t.join(e.#t),this}extension(){return this.#t.extension()}toSubpath(){if(this.#e>0)throw new Error("Cannot convert to subpath.");return this.#t}toString(){let e="";for(let r=0;r<this.#e;r++)e+="../";return e+=this.#t.toString(),e}};(e=>{let t;(s=>(s.is=i=>E.Arg.is(i)||i instanceof e||i instanceof Array&&i.every(e.Arg.is),s.expect=i=>(o((0,s.is)(i)),i),s.assert=i=>{o((0,s.is)(i))}))(t=e.Arg||={})})(v||={});var E=class{#e;constructor(e){this.#e=e??[]}static new(...e){return v.new(...e).toSubpath()}components(){return[...this.#e]}isEmpty(){return this.#e.length==0}join(e){return this.#e.push(...e.#e),this}push(e){this.#e.push(e)}pop(){this.#e.pop()}extension(){return this.#e.at(-1)?.split(".").at(-1)}toRelpath(){return v.new(this)}toString(){return this.#e.join("/")}};(e=>{let t;(s=>(s.is=i=>i===void 0||typeof i=="string"||i instanceof e||i instanceof Array&&i.every(e.Arg.is),s.expect=i=>(o((0,s.is)(i)),i),s.assert=i=>{o((0,s.is)(i))}))(t=e.Arg||={})})(E||={});var B=async(t,...e)=>{let r=[];for(let n=0;n<t.length-1;n++){let a=t[n];r.push(a);let s=e[n];r.push(s)}return r.push(t[t.length-1]),await U(...r)},U=(...t)=>g.new(...t),g=class t{#e;constructor(e){this.#e=e}static async new(...e){let r=O(await Promise.all(e.map(async function n(a){return a=await w(a),a===void 0?[]:t.Component.is(a)?a:t.is(a)?a.components:a instanceof Array?await Promise.all(a.map(n)):f()}))).reduce((n,a)=>(n.push(a),n),[]);return r=r.reduce((n,a)=>{let s=n.at(-1);return a===""||(typeof s=="string"&&typeof a=="string"?n.splice(-1,1,s+a):n.push(a)),n},[]),new t(r)}static is(e){return e instanceof t}static expect(e){return o(t.is(e)),e}static assert(e){o(t.is(e))}static async join(e,...r){let n=await U(e),a=await Promise.all(r.map(i=>U(i)));a=a.filter(i=>i.components.length>0);let s=[];for(let i=0;i<a.length;i++){i>0&&s.push(n);let l=a[i];o(l),s.push(l)}return U(...s)}get components(){return this.#e}};(r=>{let t;(a=>a.is=s=>s===void 0||e.is(s)||r.is(s)||s instanceof Array&&s.every(i=>a.is(i)))(t=r.Arg||={});let e;(a=>a.is=s=>typeof s=="string"||P.is(s))(e=r.Component||={})})(g||={});var G=async(...t)=>await A.new(...t),A=class t{#e;constructor(e){this.#e=e}static withId(e){return new t(d.Handle.withId(e))}static async new(...e){let{artifact:r,path:n}=O(await Promise.all(e.map(async function s(i){let l=await w(i);if(typeof l=="string")return{path:_(l)};if(P.is(l))return{artifact:l};if(g.is(l)){o(l.components.length<=2);let[c,p]=l.components;if(typeof c=="string"&&p===void 0)return{path:_(c)};if(P.is(c)&&p===void 0)return{artifact:c};if(P.is(c)&&typeof p=="string")return o(p.startsWith("/")),{artifact:c,path:_(p.slice(1))};throw new Error("Invalid template.")}else return t.is(l)?{artifact:await l.artifact(),path:await l.path()}:l instanceof Array?await Promise.all(l.map(s)):typeof l=="object"?{artifact:l.artifact,path:_(l.path)}:f()}))).reduce((s,{artifact:i,path:l})=>(i!==void 0?(s.artifact=i,s.path=l??_()):s.path=s.path.join(l),s),{artifact:void 0,path:_()}),a;if(r!==void 0&&!n.isEmpty())a=await B`${r}/${n.toString()}`;else if(r!==void 0)a=await B`${r}`;else if(!n.isEmpty())a=await B`${n.toString()}`;else throw new Error("Invalid symlink.");return new t(d.Handle.withObject({kind:"symlink",value:{target:a}}))}static is(e){return e instanceof t}static expect(e){return o(t.is(e)),e}static assert(e){o(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return o(e.kind==="symlink"),e.value}get handle(){return this.#e}async target(){return(await this.object()).target}async artifact(){let r=(await this.target()).components.at(0);if(P.is(r))return r}async path(){let e=await this.target(),[r,n]=e.components;if(typeof r=="string"&&n===void 0)return _(r);if(P.is(r)&&n===void 0)return _();if(P.is(r)&&typeof n=="string")return _(n.slice(1));throw new Error("Invalid template.")}async resolve(e){e=e?await G(e):void 0;let r=await e?.artifact();t.is(r)&&(r=await r.resolve());let n=e?.path(),a=await this.artifact();t.is(a)&&(a=await a.resolve());let s=await this.path();if(a!==void 0&&s.isEmpty())return a;if(a===void 0&&!s.isEmpty()){if(!b.is(r))throw new Error("Expected a directory.");return await r.tryGet((await(n??_())).parent().join(s).toSubpath().toString())}else if(a!==void 0&&!s.isEmpty()){if(!b.is(a))throw new Error("Expected a directory.");return await a.tryGet(s.toSubpath().toString())}else throw new Error("Invalid symlink.")}};var I;(r=>(r.toUrl=n=>`tangram://${m.hex.encode(m.utf8.encode(m.json.encode(n)))}/${n.value.path}`,r.fromUrl=n=>{let a=n.match(/^tangram:\/\/(.*)\/(.*)$/);o(a);let[s,i,l]=a;return o(i!==void 0),m.json.decode(m.utf8.decode(m.hex.decode(i)))}))(I||={});var ce,le=t=>{ce=t},$={};function W(...t){if(t.length===1&&typeof t[0]=="object"&&"function"in t[0]){let e=t[0],{url:r,name:n}=e,a=R.encode({url:r,name:n});o($[a]===void 0),$[a]=e.function;let s=I.fromUrl(e.url);o(s.kind==="normal");let i=j.withId(s.value.packageId);return new T(d.Handle.withObject({kind:"target",value:{host:"js-js",executable:new g([s.value.path]),package:i,name:e.name,args:[],env:{},checksum:void 0,unsafe:!1}}))}else return T.new(...t)}var ue=async(...t)=>await(await W(...t)).build(),T=class t extends globalThis.Function{#e;constructor(e){super(),this.#e=e;let r=this;return new Proxy(r,{get(n,a,s){return typeof r[a]=="function"?r[a].bind(r):r[a]},apply:async(n,a,s)=>await(await t.new(r,{args:s})).build(),getPrototypeOf:n=>Object.getPrototypeOf(r)})}static withId(e){return new t(d.Handle.withId(e))}static async new(...e){let{host:r,executable:n,package_:a,name:s,env:i,args_:l,checksum:c,unsafe_:p}=O(await Promise.all(e.map(async function x(y){let u=await w(y);return g.Arg.is(u)?{host:(await ce.env()).TANGRAM_HOST,executable:await U("/bin/sh"),args_:["-c",await U(u)]}:t.is(u)?{host:await u.host(),executable:await u.executable(),package_:await u.package(),name:await u.name_(),env:await u.env(),args_:await u.args(),checksum:await u.checksum(),unsafe_:await u.unsafe()}:u instanceof Array?await Promise.all(u.map(x)):typeof u=="object"?{host:u.host,executable:u.executable?await U(u.executable):void 0,package_:u.package,name:u.name,env:u.env,args_:u.args,checksum:u.checksum,unsafe_:u.unsafe}:f()}))).reduce((x,y)=>({host:x.host??y.host,executable:x.executable??y.executable,package_:x.package_??y.package_,name:x.name??y.name,env:{...x.env??{},...y.env??{}},args_:[...x.args_??[],...y.args_??[]],checksum:x.checksum??y.checksum,unsafe_:x.unsafe_??y.unsafe_}),{});if(!r)throw new Error("Cannot create a target without a host.");if(!n)throw new Error("Cannot create a target without an executable.");return i??={},l??=[],p??=!1,new t(d.Handle.withObject({kind:"target",value:{host:r,executable:n,package:a,name:s,env:i,args:l,checksum:c,unsafe:p}}))}static is(e){return e instanceof t}static expect(e){return o(t.is(e)),e}static assert(e){o(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return o(e.kind==="target"),e.value}get handle(){return this.#e}async host(){return(await this.object()).host}async executable(){return(await this.object()).executable}async package(){return(await this.object()).package}async name_(){return(await this.object()).name}async env(){return(await this.object()).env}async args(){return(await this.object()).args}async checksum(){return(await this.object()).checksum}async unsafe(){return(await this.object()).unsafe}async build(...e){return await Y(await t.new(this,{args:e}))}};var w=async t=>{if(t=await t,t===void 0||typeof t=="boolean"||typeof t=="number"||typeof t=="string"||t instanceof Uint8Array||t instanceof h||t instanceof b||t instanceof k||t instanceof A||t instanceof g||t instanceof j||t instanceof T)return t;if(t instanceof Array)return await Promise.all(t.map(e=>w(e)));if(typeof t=="object")return Object.fromEntries(await Promise.all(Object.entries(t).map(async([e,r])=>[e,await w(r)])));throw new Error("Invalid value to resolve.")};var M=async(...t)=>await h.new(...t),de=async(t,e)=>await h.download(t,e),h=class t{#e;constructor(e){this.#e=e}static withId(e){return new t(d.Handle.withId(e))}static async new(...e){let r=O(await Promise.all(e.map(async function n(a){let s=await w(a);return s===void 0?[]:typeof s=="string"?new t(d.Handle.withObject({kind:"blob",value:H.encode(s)})):s instanceof Uint8Array?new t(d.Handle.withObject({kind:"blob",value:s})):t.is(s)?s:s instanceof Array?await Promise.all(s.map(n)):f()})));return r.length===0?new t(d.Handle.withObject({kind:"blob",value:new Uint8Array})):r.length===1?r[0]:new t(d.Handle.withObject({kind:"blob",value:await Promise.all(r.map(async n=>[n,await n.size()]))}))}static async download(e,r){return await te(e,r)}static is(e){return e instanceof t}static expect(e){return o(t.is(e)),e}static assert(e){o(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return o(e.kind==="blob"),e.value}get handle(){return this.#e}async size(){let e=await this.object();return e instanceof Array?e.map(([r,n])=>n).reduce((r,n)=>r+n,0):e.byteLength}async bytes(){return await K(this)}async text(){return H.decode(await K(this))}async decompress(e){return await ee(this,e)}async extract(e){return await re(this,e)}};(e=>{let t;(s=>(s.is=i=>i===void 0||typeof i=="string"||i instanceof Uint8Array||e.is(i)||i instanceof Array&&i.every(s.is),s.expect=i=>(o((0,s.is)(i)),i),s.assert=i=>{o((0,s.is)(i))}))(t=e.Arg||={})})(h||={});var me=async(...t)=>await b.new(...t),b=class t{#e;constructor(e){this.#e=e}static withId(e){return new t(d.Handle.withId(e))}static async new(...e){let r=await(await Promise.all(e.map(w))).reduce(async function n(a,s){let i=await a;if(s!==void 0)if(t.is(s))for(let[l,c]of Object.entries(await s.entries())){let p=i[l];t.is(p)&&t.is(c)&&(c=await t.new(p,c)),i[l]=c}else if(s instanceof Array)for(let l of s)i=await n(Promise.resolve(i),l);else if(typeof s=="object")for(let[l,c]of Object.entries(s)){let[p,...x]=S(l).components();if(p===void 0)throw new Error("The path must have at least one component.");let y=p,u=i[y];if(t.is(u)||(u=void 0),x.length>0){let F=S(x).toString(),be=await t.new(u,{[F]:c});i[y]=be}else if(c===void 0)delete i[y];else if(h.Arg.is(c)){let F=await D(c);i[y]=F}else if(k.is(c)||A.is(c))i[y]=c;else{let F=await t.new(u,c);i[y]=F}}else return f();return i},Promise.resolve({}));return new t(d.Handle.withObject({kind:"directory",value:{entries:r}}))}static is(e){return e instanceof t}static expect(e){return o(t.is(e)),e}static assert(e){o(t.is(e))}async id(){return await this.#e.id()}async object(){let e=await this.#e.object();return o(e.kind==="directory"),e.value}get handle(){return this.#e}async get(e){let r=await this.tryGet(e);return o(r,`Failed to get the directory entry "${e}".`),r}async tryGet(e){let r=this,n=S();for(let a of S(e).components()){if(!t.is(r))return;n.push(a);let s=(await r.entries())[a];if(s===void 0)return;if(A.is(s)){let i=await s.resolve({artifact:this,path:n.toString()});if(i===void 0)return;r=i}else r=s}return r}async entries(){let e={};for await(let[r,n]of this)e[r]=n;return e}async bundle(){return await Z(this)}async*walk(){for await(let[e,r]of this)if(yield[S(e),r],t.is(r))for await(let[n,a]of r.walk())yield[S(e).join(n),a]}async*[Symbol.asyncIterator](){let e=await this.object();for(let[r,n]of Object.entries(e.entries))yield[r,n]}};var P;(n=>(n.is=a=>b.is(a)||k.is(a)||A.is(a),n.expect=a=>(o((0,n.is)(a)),a),n.assert=a=>{o((0,n.is)(a))}))(P||={});var z=class{message;location;stack;source;constructor(e,r,n,a){this.message=e,this.location=r,this.stack=n,this.source=a}},pe=(t,e)=>({callSites:e.map(n=>({typeName:n.getTypeName(),functionName:n.getFunctionName(),methodName:n.getMethodName(),fileName:n.getFileName(),lineNumber:n.getLineNumber(),columnNumber:n.getColumnNumber(),isEval:n.isEval(),isNative:n.isNative(),isConstructor:n.isConstructor(),isAsync:n.isAsync(),isPromiseAll:n.isPromiseAll(),promiseIndex:n.getPromiseIndex()}))});var ye=async t=>{let e=I.fromUrl(t.url);o(e.kind==="normal");let n=await j.withId(e.value.packageId).artifact();b.assert(n);let a=S(e.value.path).toRelpath().parent().join(t.path).toSubpath().toString();return await n.get(a)};var q=(...t)=>{let e=t.map(r=>xe(r)).join(" ");ne(e)},xe=t=>L(t,new WeakSet),L=(t,e)=>{switch(typeof t){case"string":return`"${t}"`;case"number":return t.toString();case"boolean":return t?"true":"false";case"undefined":return"undefined";case"object":return t===null?"null":fe(t,e);case"function":return`(function "${t.name??"(anonymous)"}")`;case"symbol":return"(symbol)";case"bigint":return t.toString()}},fe=(t,e)=>{if(e.has(t))return"(circular)";if(e.add(t),t instanceof Array)return`[${t.map(r=>L(r,e)).join(", ")}]`;if(t instanceof Error)return t.message;if(t instanceof Promise)return"(promise)";if(h.is(t))return`(tg.blob ${C(t.handle,e)})`;if(b.is(t))return`(tg.directory ${C(t.handle,e)})`;if(k.is(t))return`(tg.file ${C(t.handle,e)})`;if(A.is(t))return`(tg.symlink ${C(t.handle,e)})`;if(g.is(t))return`(tg.template "${t.components.map(n=>typeof n=="string"?n:`\${${L(n,e)}}`).join("")}")`;if(j.is(t))return`(tg.package "${C(t.handle,e)}")`;if(T.is(t))return`(tg.target "${C(t.handle,e)}")`;{let r="";t.constructor!==void 0&&t.constructor.name!=="Object"&&(r=`${t.constructor.name} `);let n=Object.entries(t).map(([a,s])=>`${a}: ${L(s,e)}`);return`${r}{ ${n.join(", ")} }`}},C=(t,e)=>{let{id:r,object:n}=t.state;return r!==void 0?r:n!==void 0?fe(n,e):f()};var he=async t=>{let e=await t.package();o(e);let r=await e.id(),a=(await t.executable()).components[0];o(typeof a=="string");let s={kind:"normal",value:{packageId:r,path:a}},i=I.toUrl(s);await import(i);let l=await t.name_();if(!l)throw new Error("The target must have a name.");let c=R.encode({url:i,name:l}),p=$[c];if(!p)throw new Error("Failed to find the function.");le(t);let x=await t.args();return await p(...x)};var ge=t=>{if(typeof t=="string")return t;{let{arch:e,os:r}=t;return`${e}-${r}`}},J;(n=>(n.is=a=>a==="aarch64-darwin"||a==="aarch64-linux"||a==="js-js"||a==="x86_64-darwin"||a==="x86_64-linux",n.arch=a=>{switch(a){case"aarch64-darwin":case"aarch64-linux":return"aarch64";case"js-js":return"js";case"x86_64-linux":case"x86_64-darwin":return"x86_64";default:throw new Error("Invalid system.")}},n.os=a=>{switch(a){case"aarch64-darwin":case"x86_64-darwin":return"darwin";case"js-js":return"js";case"x86_64-linux":case"aarch64-linux":return"linux";default:throw new Error("Invalid system.")}}))(J||={});var Q;(n=>(n.is=a=>a===void 0||typeof a=="boolean"||typeof a=="number"||typeof a=="string"||a instanceof Uint8Array||a instanceof h||a instanceof b||a instanceof k||a instanceof A||a instanceof g||a instanceof j||a instanceof T||a instanceof Array||typeof a=="object",n.expect=a=>(o((0,n.is)(a)),a),n.assert=a=>{o((0,n.is)(a))}))(Q||={});Object.defineProperties(Error,{prepareStackTrace:{value:pe}});var ke={log:q};Object.defineProperties(globalThis,{console:{value:ke}});var je={Artifact:P,Blob:h,Directory:b,Error:z,File:k,Object_:d,Package:j,Symlink:A,System:J,Target:T,Template:g,Value:Q,assert:o,blob:M,build:ue,directory:me,download:de,encoding:V,file:D,include:ye,log:q,main:he,resolve:w,symlink:G,system:ge,target:W,template:U,unimplemented:X,unreachable:f};Object.defineProperties(globalThis,{tg:{value:je},t:{value:B}});})();
//# sourceMappingURL=runtime.js.map
