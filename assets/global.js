"use strict";(()=>{var S=async e=>await syscall("add_blob",e),N=async e=>await syscall("get_blob",e);var y=(e,t)=>{if(!e)throw t=t??"Failed assertion.",new Error(t)};var G=e=>e instanceof Uint8Array||typeof e=="string"||e instanceof c,U=async(e,t)=>{e=await e;let r,a;return e instanceof Uint8Array?r=await S(e):typeof e=="string"?r=await S(syscall("encode_utf8",e)):(r=e.blobHash(),a=e.executable()),a=t?.executable??a,new c(r,{executable:a})},D=e=>e instanceof c,c=class{#e;#t;constructor(t,r){this.#e=t,this.#t=r?.executable??!1}static async fromHash(t){let r=await u(t);return y(D(r)),r}async serialize(){let t=this.#e,r=this.#t;return{blobHash:t,executable:r}}static async deserialize(t){let r=t.blobHash,a=t.executable;return new c(r,{executable:a})}hash(){return P(this)}blobHash(){return this.#e}executable(){return this.#t}async getBytes(){return await N(this.#e)}async getString(){let t=await this.getBytes();return syscall("decode_utf8",t)}};var d=e=>new O(e),W=e=>e instanceof O,O=class{#e;constructor(t){if(typeof t=="string"){if(this.#e=[],t.startsWith("/"))throw new Error("Absolute paths are not allowed.");let r=t.split("/");for(let a of r){if(a==="")throw new Error("Empty path components are not allowed.");a==="."||(a===".."?this.#e.push({kind:"parent_dir"}):this.#e.push({kind:"normal",value:a}))}}else t instanceof Array?this.#e=t:this.#e=t.components()}components(){return[...this.#e]}push(t){if(t.kind==="parent_dir"){let r=this.#e.at(-1);r===void 0||r.kind==="parent_dir"?this.#e.push(t):this.#e.pop()}else this.#e.push(t)}parent(){let t=d(this);return t.push({kind:"parent_dir"}),t}join(t){let r=d(this);for(let a of d(t).components())r.push(a);return r}toString(){let t=this.#e.map(a=>{switch(a.kind){case"parent_dir":return"..";case"normal":return a.value}}).join("/"),r=this.#e[0];return r===void 0?".":r.kind==="parent_dir"?t:`./${t}`}};var _=e=>new m(e),q=e=>e instanceof m,m=class{#e;constructor(t){this.#e=t}async serialize(){return{name:this.#e}}static async deserialize(t){let r=t.name;return new m(r)}name(){return this.#e}};var J=async e=>{let t=await w(await e.artifact),r=C(e.path)?e.path:d(e.path);return new p({artifactHash:t,path:r})},E=e=>e instanceof p,p=class{#e;#t;constructor(t){this.#e=t.artifactHash,this.#t=t.path}static async fromHash(t){let r=await u(t);return y(E(r)),r}async serialize(){let t=this.#e,r=this.#t?.toString();return{artifactHash:t,path:r}}static async deserialize(t){let r=t.artifactHash,a=C(t.path)?t.path:d(t.path);return new p({artifactHash:r,path:a})}hash(){return P(this)}artifactHash(){return this.#e}path(){return this.#t}async getArtifact(){return await u(this.#e)}};var Q=e=>new f(e),I=e=>e instanceof f,f=class{#e;constructor(t){this.#e=t}static async fromHash(t){let r=await u(t);return y(I(r)),r}async serialize(){return{target:this.#e}}static async deserialize(t){return new f(t.target)}hash(){return P(this)}target(){return this.#e}};var A=async e=>{if(e=await e,e==null||typeof e=="boolean"||typeof e=="number"||typeof e=="string"||e instanceof l||e instanceof c||e instanceof f||e instanceof p||e instanceof m||e instanceof o)return e;if(e instanceof Array)return await Promise.all(e.map(t=>A(t)));if(typeof e=="object")return Object.fromEntries(await Promise.all(Object.entries(e).map(async([t,r])=>[t,await A(r)])));if(typeof e=="function")return await A(e());throw new Error("Invalid value to resolve.")};var X=async(e,...t)=>{let r=[];for(let a=0;a<e.length-1;a++){let i=e[a];r.push(i);let n=t[a];r.push(n)}return r.push(e[e.length-1]),await b(r)},b=async e=>{let t=await A(e),r=[],a=i=>{i instanceof Array?i.forEach(a):i instanceof o?r.push(...i.components()):r.push(i)};return a(t),new o(r)},Y=e=>e instanceof o,o=class{#e;constructor(t){this.#e=t}async serialize(){return{components:await Promise.all(this.#e.map(async r=>await oe(r)))}}static async deserialize(t){return new o(await Promise.all(t.components.map(async r=>await le(r))))}components(){return[...this.#e]}render(t){return this.#e.map(t).join("")}},oe=async e=>{if(typeof e=="string")return{kind:"string",value:e};if(x(e))return{kind:"artifact",value:await w(e)};if(e instanceof m)return{kind:"placeholder",value:await e.serialize()};throw new Error("Invalid template component.")},le=async e=>{switch(e.kind){case"string":return await e.value;case"artifact":return await u(e.value);case"placeholder":return await m.deserialize(e.value)}};var C=e=>e==null;var k=async e=>{if(e==null)return{kind:"null",value:e};if(typeof e=="boolean")return{kind:"bool",value:e};if(typeof e=="number")return{kind:"number",value:e};if(typeof e=="string")return{kind:"string",value:e};if(x(e))return{kind:"artifact",value:await w(e)};if(e instanceof m)return{kind:"placeholder",value:await e.serialize()};if(e instanceof o)return{kind:"template",value:await e.serialize()};if(e instanceof Array)return{kind:"array",value:await Promise.all(e.map(r=>k(r)))};if(typeof e=="object")return{kind:"map",value:Object.fromEntries(await Promise.all(Object.entries(e).map(async([r,a])=>[r,await k(a)])))};throw new Error("Failed to serialize the value.")},g=async e=>{switch(e.kind){case"null":return e.value;case"bool":return e.value;case"number":return e.value;case"string":return e.value;case"artifact":return await u(e.value);case"placeholder":return await m.deserialize(e.value);case"template":return await o.deserialize(e.value);case"array":return await Promise.all(e.value.map(t=>g(t)));case"map":return Object.fromEntries(await Promise.all(Object.entries(e.value).map(async([t,r])=>[t,await g(r)])))}};var v=async(...e)=>{let t=new Map;for(let r of e)if(r=await r,!C(r))if(r instanceof l)for(let[a,i]of r)t.set(a,i);else for(let[a,i]of Object.entries(r)){let[n,...h]=d(a).components();if(n===void 0)throw new Error("The path must have at least one component.");if(n.kind!=="normal")throw new Error("Invalid path component.");let s=n.value;if(h.length>0){let j=d(h).toString(),L=t.get(s),M;L!==void 0&&(M=await u(L),M instanceof l||(M=void 0));let se=await v(M,{[j]:i});t.set(s,await w(se))}else i=await i,C(i)?t.delete(s):G(i)?t.set(s,await w(await U(i))):x(i)?t.set(s,await w(i)):t.set(s,await w(await v(i)))}return new l(t)},B=e=>e instanceof l,l=class{#e;constructor(t){this.#e=t}static async fromHash(t){let r=await u(t);return y(B(r)),r}async serialize(){return{entries:Object.fromEntries(Array.from(this.#e.entries()))}}static async deserialize(t){let r=new Map(Object.entries(t.entries));return new l(r)}hash(){return P(this)}async get(t){let r=await this.tryGet(t);return y(r!==null,`Failed to get directory entry "${t}".`),r}async tryGet(t){let r=this;for(let a of d(t).components()){if(y(a.kind==="normal"),!(r instanceof l))return null;let i=r.#e.get(a.value);if(!i)return null;r=await u(i)}return r}async getEntries(){let t={};for await(let[r,a]of this)t[r]=a;return t}*[Symbol.iterator](){for(let[t,r]of this.#e)yield[t,r]}async*[Symbol.asyncIterator](){for(let t of this.#e.keys())yield[t,await this.get(t)]}};var x=e=>e instanceof l||e instanceof c||e instanceof f||e instanceof p,w=async e=>await syscall("add_artifact",await Z(e)),u=async e=>await ce(await syscall("get_artifact",e)),P=async e=>syscall("get_artifact_hash",await Z(e)),Z=async e=>{if(e instanceof l)return{kind:"directory",value:await e.serialize()};if(e instanceof c)return{kind:"file",value:await e.serialize()};if(e instanceof f)return{kind:"symlink",value:await e.serialize()};if(e instanceof p)return{kind:"reference",value:await e.serialize()};throw new Error("Unknown artifact type")},ce=async e=>{switch(e.kind){case"directory":return await l.deserialize(e.value);case"file":return await c.deserialize(e.value);case"symlink":return await f.deserialize(e.value);case"reference":return await p.deserialize(e.value)}};var ee=(e,t)=>syscall("checksum",e,t);var F=new Map;var te=e=>{let t=syscall("get_current_package_instance_hash"),r=syscall("get_current_export_name");return new T({packageInstanceHash:t,name:r,implementation:e})};var T=class extends globalThis.Function{packageInstanceHash;name;implementation;constructor(t){return super(),this.packageInstanceHash=t.packageInstanceHash,this.name=t.name,this.implementation=t.implementation,new Proxy(this,{apply:(r,a,i)=>r._call(...i)})}async serialize(){let t=this.packageInstanceHash,r=this.name?.toString();return{packageInstanceHash:t,name:r}}static async deserialize(t){let r=t.packageInstanceHash,a=t.name;return new T({packageInstanceHash:r,name:a})}async _call(...t){let r=new Map(await F.entries()),a=await Promise.all(t.map(A));return await re({function:this,context:r,args:a})}async run(t,r){y(this.implementation,"This function does not have an implementation.");for(let[h,s]of Object.entries(r))F.set(h,await g(s));let a=await Promise.all(t.map(g)),i=await this.implementation(...a);return await k(i)}};var re=async e=>{let t=e.function,r=e.context??new Map,a=e.args??[];return await new H({function:t,context:r,args:a}).run()};var H=class{#e;#t;#r;constructor(t){this.#e=t.function,this.#t=t.context,this.#r=t.args}async serialize(){let t=await this.#e.serialize(),r=Object.fromEntries(await Promise.all(Array.from(this.#t.entries()).map(async([i,n])=>[i,await k(n)]))),a=await Promise.all(this.#r.map(i=>k(i)));return{function:t,context:r,args:a}}static async deserialize(t){let r=await T.deserialize(t.function),a=new Map(await Promise.all(Object.entries(t.context).map(async([n,h])=>[n,await g(h)]))),i=await Promise.all(t.args.map(n=>g(n)));return new H({function:r,context:a,args:i})}async run(){return await R(this)}};var ae=async e=>{let t=await A(e),r=t.system,a=Object.fromEntries(await Promise.all(Object.entries(t.env??{}).map(async([s,j])=>[s,await b(j)]))),i=await b(t.command),n=await Promise.all((t.args??[]).map(async s=>await b(s))),h=t.unsafe??!1;return await new z({system:r,env:a,command:i,args:n,unsafe:h}).run()},ie=_("output"),z=class{#e;#t;#r;#a;#i;constructor(t){this.#e=t.system,this.#t=t.env,this.#r=t.command,this.#a=t.args,this.#i=t.unsafe}async serialize(){let t=this.#e,r=Object.fromEntries(await Promise.all(Object.entries(this.#t).map(async([h,s])=>[h,await s.serialize()]))),a=await this.#r.serialize(),i=await Promise.all(this.#a.map(h=>h.serialize())),n=this.#i;return{system:t,env:r,command:a,args:i,unsafe:n}}static async deserialize(t){let r=t.system,a=Object.fromEntries(await Promise.all(Object.entries(t.env).map(async([s,j])=>[s,await o.deserialize(j)]))),i=await o.deserialize(t.command),n=await Promise.all(t.args.map(s=>o.deserialize(s))),h=t.unsafe;return new z({system:r,env:a,command:i,args:n,unsafe:h})}async run(){return await R(this)}};var R=async e=>{let t=await me(e),r=await syscall("run",t);return await g(r)},me=async e=>await syscall("add_operation",await ue(e));var ue=async e=>{if(e instanceof V)return{kind:"download",value:await e.serialize()};if(e instanceof z)return{kind:"process",value:await e.serialize()};if(e instanceof H)return{kind:"call",value:await e.serialize()};throw new Error("Cannot serialize operation.")};var ne=async e=>await new V(e).run();var V=class{#e;#t;#r;#a;constructor(t){this.#e=t.url,this.#t=t.unpack??!1,this.#r=t.checksum??null,this.#a=t.unsafe??!1}async serialize(){return{url:this.#e,unpack:this.#t,checksum:this.#r,unsafe:this.#a}}static async deserialize(t){return new V({url:t.url,unpack:t.unpack,checksum:t.checksum,unsafe:t.unsafe})}async run(){return await R(this)}};var K=(...e)=>{let t=e.map(r=>pe(r)).join(" ");syscall("log",t)},pe=e=>$(e,new Set),$=(e,t)=>{switch(typeof e){case"string":return`"${e}"`;case"number":return e.toString();case"boolean":return e?"true":"false";case"undefined":return"undefined";case"object":return fe(e,t);case"function":return`[function ${e.name??"(anonymous)"}]`;case"symbol":return"[symbol]";case"bigint":return e.toString()}},fe=(e,t)=>{if(e===null)return"null";if(t.has(e))return"[circular]";if(t.add(e),e instanceof Array)return`[${e.map(r=>$(r,t)).join(", ")}]`;if(e instanceof Error)return e.stack??"";if(e instanceof Promise)return"[promise]";{let r="";e.constructor!==void 0&&e.constructor.name!=="Object"&&(r=`${e.constructor.name} `);let a=Object.entries(e).map(([i,n])=>`${i}: ${$(n,t)}`);return`${r}{ ${a.join(", ")} }`}};var he={log:K},ye={Directory:l,File:c,Path:O,Placeholder:m,Reference:p,Symlink:f,Template:o,checksum:ee,context:F,directory:v,download:ne,file:U,function:te,isArtifact:x,isDirectory:B,isFile:D,isPath:W,isPlaceholder:q,isReference:E,isSymlink:I,isTemplate:Y,log:K,output:ie,path:d,placeholder:_,process:ae,reference:J,resolve:A,symlink:Q,template:b};Object.defineProperties(globalThis,{console:{value:he},t:{value:X},tg:{value:ye}});})();
