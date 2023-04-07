"use strict";(()=>{var A=(t,e)=>{if(!t)throw new Error(e??"Failed assertion.")},x=t=>{throw new Error(t??"Reached unreachable code.")};var I={bundle:async t=>{try{return await syscall("artifact_bundle",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},get:async t=>{try{return await syscall("artifact_get",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}};var D={bytes:async t=>{try{return await syscall("blob_bytes",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},new:async t=>{try{return await syscall("blob_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},text:async t=>{try{return await syscall("blob_text",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},G={new:async(t,e,r)=>{try{return await syscall("call_new",t,e,r)}catch(s){throw new Error("The syscall failed.",{cause:s})}}},M=()=>{try{return syscall("caller")}catch(t){throw new Error("The syscall failed.",{cause:t})}},Z={new:async(t,e,r,s)=>{try{return await syscall("download_new",t,e,r,s)}catch(a){throw new Error("The syscall failed.",{cause:a})}}},J={new:async t=>{try{return await syscall("directory_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},Q={new:async(t,e,r)=>{try{return await syscall("file_new",t,e,r)}catch(s){throw new Error("The syscall failed.",{cause:s})}}};var X=async(t,e)=>{try{return await syscall("include",t,e)}catch(r){throw new Error("The syscall failed.",{cause:r})}};var Y=t=>{try{return syscall("log",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},R={get:async t=>{try{return await syscall("operation_get",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},run:async t=>{try{return await syscall("operation_run",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},ee={new:async(t,e,r,s,a,n,l,y)=>{try{return await syscall("process_new",t,e,r,s,a,n,l,y)}catch(w){throw new Error("The syscall failed.",{cause:w})}}},te={new:async t=>{try{return await syscall("symlink_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}};var N=async t=>{let e=await d(t),r,s,a;if(h.isBlobArg(e))r=await F(e),s=!1,a=[];else{if(p.isFile(e))return e;r=await F(e.blob),s=e.executable??!1,a=e.references??[]}return p.fromSyscall(await Q.new(r.toSyscall(),s,a.map(n=>o.toSyscall(n))))},p=class{#e;#t;#r;#s;constructor(e){this.#e=e.hash,this.#t=e.blob,this.#r=e.executable,this.#s=e.references}static isFile(e){return e instanceof p}toSyscall(){return{hash:this.#e,blob:this.#t.toSyscall(),executable:this.#r,references:this.#s}}static fromSyscall(e){return new p({hash:e.hash,blob:h.fromSyscall(e.blob),executable:e.executable,references:e.references})}hash(){return this.#e}blob(){return this.#t}executable(){return this.#r}async references(){return await Promise.all(this.#s.map(o.get))}async bytes(){return await this.blob().bytes()}async text(){return await this.blob().text()}};var g=(...t)=>{let e=[],r=a=>{if(typeof a=="string")for(let n of a.split("/"))n===""||n==="."||(n===".."?e.push({kind:"parent"}):e.push({kind:"normal",value:n}));else if(c.Component.isPathComponent(a))e.push(a);else if(a instanceof c)e.push(...a.components());else if(a instanceof Array)for(let n of a)r(n)};for(let a of t)r(a);let s=new c;for(let a of e)s.push(a);return s},c=class{#e;constructor(e=[]){this.#e=e}static isPathArg(e){return typeof e=="string"||c.Component.isPathComponent(e)||e instanceof c||e instanceof Array&&e.every(c.isPathArg)}static isPath(e){return e instanceof c}toSyscall(){return this.toString()}static fromSyscall(e){return g(e)}components(){return[...this.#e]}push(e){if(e.kind==="parent"){let r=this.#e.at(-1);r===void 0||r.kind==="parent"?this.#e.push(e):this.#e.pop()}else this.#e.push(e)}join(e){let r=g(this);for(let s of g(e).components())r.push(s);return r}diff(e){let r=g(e),s=g(this);for(;;){let n=r.#e.at(0),l=s.#e.at(0);if(n&&l&&c.Component.equal(n,l))r.#e.shift(),s.#e.shift();else break}if(r.#e.at(0)?.kind==="parent")throw new Error(`There is no valid path from "${r}" to "${s}".`);return g(Array.from({length:r.#e.length},()=>({kind:"parent"})),s)}toString(){return this.#e.map(e=>{switch(e.kind){case"parent":return"..";case"normal":return e.value}}).join("/")}};(e=>{let t;(a=>(a.isPathComponent=n=>typeof n=="object"&&n!==null&&"kind"in n&&(n.kind==="parent"||n.kind==="normal"),a.equal=(n,l)=>n.kind===l.kind&&(n.kind==="normal"&&l.kind==="normal"?n.value===l.value:!0)))(t=e.Component||={})})(c||={});var $=t=>new b(t),b=class{#e;constructor(e){this.#e=e}static isPlaceholder(e){return e instanceof b}toSyscall(){return{name:this.#e}}static fromSyscall(e){let r=e.name;return new b(r)}name(){return this.#e}};var U=async(t,...e)=>{let r=[];for(let s=0;s<t.length-1;s++){let a=t[s];r.push(a);let n=e[s];r.push(n)}return r.push(t[t.length-1]),await S(...r)},S=async(...t)=>{let e=[],r=a=>{if(i.Component.isTemplateComponent(a))e.push(a);else if(a instanceof c)e.push(a.toString());else if(a instanceof i)e.push(...a.components());else if(a instanceof Array)for(let n of a)r(n)};for(let a of await Promise.all(t.map(d)))r(a);let s=[];for(let a of e){let n=s.at(-1);a!==""&&(typeof n=="string"&&typeof a=="string"?s.splice(-1,1,n+a):s.push(a))}return e=s,new i(e)},i=class{#e;constructor(e){this.#e=e}static isTemplate(e){return e instanceof i}static async join(e,...r){let s=await S(e),a=await Promise.all(r.map(l=>S(l))),n=[];for(let l=0;l<a.length;l++){l>0&&n.push(s);let y=a[l];A(y),n.push(y)}return S(...n)}toSyscall(){return{components:this.#e.map(r=>i.Component.toSyscall(r))}}static fromSyscall(e){let r=e.components.map(s=>i.Component.fromSyscall(s));return new i(r)}components(){return[...this.#e]}};(e=>{let t;(n=>(n.isTemplateComponent=l=>typeof l=="string"||o.isArtifact(l)||l instanceof b,n.toSyscall=l=>typeof l=="string"?{kind:"string",value:l}:o.isArtifact(l)?{kind:"artifact",value:o.toSyscall(l)}:l instanceof b?{kind:"placeholder",value:l.toSyscall()}:x(),n.fromSyscall=l=>{switch(l.kind){case"string":return l.value;case"artifact":return o.fromSyscall(l.value);case"placeholder":return b.fromSyscall(l.value);default:return x()}}))(t=e.Component||={})})(i||={});var re=async t=>{let e=await d(t),r,s;if(typeof e=="string")s=e;else if(c.isPath(e))s=e.toString();else if(o.isArtifact(e))r=e;else if(e instanceof i){A(e.components().length<=2);let[n,l]=e.components();if(typeof n=="string"&&l===void 0)s=n;else if(o.isArtifact(n)&&l===void 0)r=n;else if(o.isArtifact(n)&&typeof l=="string")r=n,A(l.startsWith("/")),s=l.slice(1);else throw new Error("Invalid template.")}else{if(e instanceof f)return e;if(typeof e=="object"){r=e.artifact;let n=e.path;typeof n=="string"?s=n:c.isPath(n)&&(s=n.toString())}}let a;return r!==void 0&&s!==void 0?a=await U`${r}/${s}`:r!==void 0&&s===void 0?a=await U`${r}`:r===void 0&&s!==void 0?a=await U`${s}`:a=await U``,f.fromSyscall(await te.new(a.toSyscall()))},f=class{#e;#t;constructor(e){this.#e=e.hash,this.#t=e.target}static isSymlink(e){return e instanceof f}toSyscall(){let e=this.#e,r=this.#t.toSyscall();return{hash:e,target:r}}static fromSyscall(e){let r=e.hash,s=i.fromSyscall(e.target);return new f({hash:r,target:s})}hash(){return this.#e}target(){return this.#t}artifact(){let e=this.#t.components().at(0);if(o.isArtifact(e))return e}path(){let[e,r]=this.#t.components();if(typeof e=="string"&&r===void 0)return g(e);if(o.isArtifact(e)&&r===void 0)return g();if(o.isArtifact(e)&&typeof r=="string")return g(r);throw new Error("Invalid template.")}async resolve(){let e=this;for(;f.isSymlink(e);){let r=e.artifact(),s=e.path();if(u.isDirectory(r))e=await r.get(s);else if(p.isFile(r))A(s.components().length===0),e=r;else if(f.isSymlink(r))A(s.components().length===0),e=r;else throw new Error("Cannot resolve a symlink without an artifact in its target.")}return e}};var d=async t=>{if(t=await t,t==null||typeof t=="boolean"||typeof t=="number"||typeof t=="string"||t instanceof Uint8Array||t instanceof c||t instanceof h||t instanceof u||t instanceof p||t instanceof f||t instanceof b||t instanceof i)return t;if(t instanceof Array)return await Promise.all(t.map(e=>d(e)));if(typeof t=="object")return Object.fromEntries(await Promise.all(Object.entries(t).map(async([e,r])=>[e,await d(r)])));throw new Error("Invalid value to resolve.")};var F=async t=>{let e=await d(t),r;if(e instanceof Uint8Array||typeof e=="string")r=e;else return e;return h.fromSyscall(await D.new(r))},h=class{#e;constructor(e){this.#e=e.hash}static isBlobArg(e){return e instanceof Uint8Array||typeof e=="string"||e instanceof h}static isBlob(e){return e instanceof h}toSyscall(){return{hash:this.#e}}static fromSyscall(e){let r=e.hash;return new h({hash:r})}hash(){return this.#e}async bytes(){return await D.bytes(this.toSyscall())}async text(){return await D.text(this.toSyscall())}};var _;(e=>e.isNullish=r=>r==null)(_||={});var m={isValue:t=>t==null||typeof t=="boolean"||typeof t=="number"||typeof t=="string"||t instanceof Uint8Array||t instanceof c||t instanceof h||t instanceof u||t instanceof p||t instanceof f||t instanceof b||t instanceof i||t instanceof Array||typeof t=="object",toSyscall:t=>t==null?{kind:"null",value:t}:typeof t=="boolean"?{kind:"bool",value:t}:typeof t=="number"?{kind:"number",value:t}:typeof t=="string"?{kind:"string",value:t}:t instanceof Uint8Array?{kind:"bytes",value:t}:t instanceof c?{kind:"path",value:t.toSyscall()}:t instanceof h?{kind:"blob",value:t.toSyscall()}:o.isArtifact(t)?{kind:"artifact",value:o.toSyscall(t)}:t instanceof b?{kind:"placeholder",value:t.toSyscall()}:t instanceof i?{kind:"template",value:t.toSyscall()}:t instanceof Array?{kind:"array",value:t.map(r=>m.toSyscall(r))}:typeof t=="object"?{kind:"object",value:Object.fromEntries(Object.entries(t).map(([r,s])=>[r,m.toSyscall(s)]))}:x(),fromSyscall:t=>{switch(t.kind){case"null":return t.value;case"bool":return t.value;case"number":return t.value;case"string":return t.value;case"bytes":return t.value;case"path":return c.fromSyscall(t.value);case"blob":return h.fromSyscall(t.value);case"artifact":return o.fromSyscall(t.value);case"placeholder":return b.fromSyscall(t.value);case"template":return i.fromSyscall(t.value);case"array":return t.value.map(e=>m.fromSyscall(e));case"object":return Object.fromEntries(Object.entries(t.value).map(([e,r])=>[e,m.fromSyscall(r)]));default:return x()}}};var B=async(...t)=>{let e=new Map;for(let r of await Promise.all(t.map(d)))if(!_.isNullish(r)){if(r instanceof u)for(let[s,a]of await r.entries()){let n=e.get(s);n instanceof u&&a instanceof u&&(a=await B(n,a)),e.set(s,a)}else if(typeof r=="object")for(let[s,a]of Object.entries(r)){let[n,...l]=g(s).components();if(n===void 0)throw new Error("The path must have at least one component.");if(n.kind!=="normal")throw new Error("Invalid path component.");let y=n.value,w=e.get(y);if(w instanceof u||(w=void 0),l.length>0){let k=g(l).toString(),O=await B(w,{[k]:a});e.set(y,O)}else if(_.isNullish(a))e.delete(y);else if(h.isBlobArg(a)){let k=await N(a);e.set(y,k)}else if(p.isFile(a)||f.isSymlink(a))e.set(y,a);else{let k=await B(w,a);e.set(y,k)}}}return u.fromSyscall(await J.new(new Map(Array.from(e,([r,s])=>[r,o.toSyscall(s)]))))},u=class{#e;#t;constructor(e){this.#e=e.hash,this.#t=e.entries}static isDirectory(e){return e instanceof u}toSyscall(){return{hash:this.#e,entries:Object.fromEntries(this.#t)}}static fromSyscall(e){let r=e.hash,s=new Map(Object.entries(e.entries));return new u({hash:r,entries:s})}hash(){return this.#e}async get(e){let r=await this.tryGet(e);return A(r,`Failed to get the directory entry "${e}".`),r}async tryGet(e){let r=this;for(let s of g(e).components()){if(A(s.kind==="normal"),!(r instanceof u))return;let a=r.#t.get(s.value);if(!a)return;r=await o.get(a)}return r}async entries(){let e=new Map;for await(let[r,s]of this)e.set(r,s);return e}async bundle(){let e=o.fromSyscall(await I.bundle(o.toSyscall(this)));return A(u.isDirectory(e)),e}async*walk(){for await(let[e,r]of this)if(yield[g(e),r],u.isDirectory(r))for await(let[s,a]of r.walk())yield[g(e).join(s),a]}*[Symbol.iterator](){for(let[e,r]of this.#t)yield[e,r]}async*[Symbol.asyncIterator](){for(let e of this.#t.keys())yield[e,await this.get(e)]}};var o;(a=>(a.isArtifact=n=>n instanceof u||n instanceof p||n instanceof f,a.get=async n=>a.fromSyscall(await I.get(n)),a.toSyscall=n=>n instanceof u?{kind:"directory",value:n.toSyscall()}:n instanceof p?{kind:"file",value:n.toSyscall()}:n instanceof f?{kind:"symlink",value:n.toSyscall()}:x(),a.fromSyscall=n=>{switch(n.kind){case"directory":return u.fromSyscall(n.value);case"file":return p.fromSyscall(n.value);case"symlink":return f.fromSyscall(n.value);default:return x()}}))(o||={});var z=new Map;var se=t=>{let{module:e,line:r}=M();A(e.kind==="normal");let s=e.value.packageInstanceHash,a;if(r.startsWith("export default "))a="default";else if(r.startsWith("export let ")){let n=r.match(/^export let ([a-zA-Z0-9]+)\b/)?.at(1);if(!n)throw new Error("Invalid use of tg.function.");a=n}else throw new Error("Invalid use of tg.function.");return new T({packageInstanceHash:s,name:a,f:t})},T=class extends globalThis.Function{packageInstanceHash;name;f;constructor(e){return super(),this.packageInstanceHash=e.packageInstanceHash,this.name=e.name,this.f=e.f,new Proxy(this,{apply:async(r,s,a)=>{let n=await Promise.all(a.map(d));return await K({function:r,args:n})}})}static isFunction(e){return e instanceof T}toSyscall(){let e=this.packageInstanceHash,r=this.name?.toString();return{packageInstanceHash:e,name:r}}static fromSyscall(e){let r=e.packageInstanceHash,s=e.name;return new T({packageInstanceHash:r,name:s})}async run(e,r){for(let[n,l]of Object.entries(e))z.set(n,m.fromSyscall(l));let s=r.map(m.fromSyscall);A(this.f);let a=await this.f(...s);return m.toSyscall(a)}};var ae=async t=>await C.fromSyscall(await Z.new(t.url,t.unpack??!1,t.checksum??null,t.unsafe??!1)).run(),C=class{#e;#t;#r;#s;#a;constructor(e){this.#e=e.hash,this.#t=e.url,this.#r=e.unpack??!1,this.#s=e.checksum??null,this.#a=e.unsafe??!1}static isDownload(e){return e instanceof C}hash(){return this.#e}toSyscall(){return{hash:this.#e,url:this.#t,unpack:this.#r,checksum:this.#s,unsafe:this.#a}}static fromSyscall(e){return new C({hash:e.hash,url:e.url,unpack:e.unpack,checksum:e.checksum,unsafe:e.unsafe})}async run(){let e=await R.run(E.toSyscall(this));return m.fromSyscall(e)}};var ne=async t=>{let e=await d(t),r=e.system,s=await S(e.executable),a=Object.fromEntries(await Promise.all(Object.entries(e.env??{}).map(async([H,ue])=>[H,await S(ue)]))),n=await Promise.all((e.args??[]).map(async H=>await S(H))),l=e.checksum??null,y=e.unsafe??!1,w=e.network??!1,k=e.hostPaths??[];return await V.fromSyscall(await ee.new(r,s.toSyscall(),a,n.map(H=>H.toSyscall()),l,y,w,k)).run()},le=$("output"),V=class{#e;#t;#r;#s;#a;#n;#l;#o;#i;constructor(e){this.#e=e.hash,this.#t=e.system,this.#r=e.executable,this.#s=e.env,this.#a=e.args,this.#n=e.checksum,this.#l=e.unsafe,this.#o=e.network,this.#i=e.hostPaths}hash(){return this.#e}toSyscall(){let e=this.#e,r=this.#t,s=this.#r.toSyscall(),a=Object.fromEntries(Object.entries(this.#s).map(([O,j])=>[O,j.toSyscall()])),n=this.#a.map(O=>O.toSyscall()),l=this.#n,y=this.#l,w=this.#o,k=this.#i;return{hash:e,system:r,executable:s,env:a,args:n,checksum:l,unsafe:y,network:w,hostPaths:k}}static fromSyscall(e){let r=e.hash,s=e.system,a=i.fromSyscall(e.executable),n=Object.fromEntries(Object.entries(e.env).map(([j,H])=>[j,i.fromSyscall(H)])),l=e.args.map(j=>i.fromSyscall(j)),y=e.checksum,w=e.unsafe,k=e.network,O=e.hostPaths;return new V({hash:r,system:s,executable:a,env:n,args:l,checksum:y,unsafe:w,network:k,hostPaths:O})}async run(){let e=await R.run(E.toSyscall(this));return m.fromSyscall(e)}};var E;(s=>(s.isOperation=a=>a instanceof v||a instanceof C||a instanceof V,s.toSyscall=a=>a instanceof C?{kind:"download",value:a.toSyscall()}:a instanceof V?{kind:"process",value:a.toSyscall()}:a instanceof v?{kind:"call",value:a.toSyscall()}:x(),s.fromSyscall=(a,n)=>{switch(n.kind){case"download":return C.fromSyscall(n.value);case"process":return V.fromSyscall(n.value);case"call":return v.fromSyscall(n.value);default:return x()}}))(E||={});var K=async t=>{let e=t.function.toSyscall(),r=Object.fromEntries(Object.entries(t.env??{}).map(([l,y])=>[l,m.toSyscall(y)])),s=(t.args??[]).map(l=>m.toSyscall(l));return await v.fromSyscall(await G.new(e,r,s)).run()},v=class{#e;#t;#r;#s;constructor(e){this.#e=e.hash,this.#t=e.function,this.#r=e.env,this.#s=e.args}static isCall(e){return e instanceof v}hash(){return this.#e}toSyscall(){let e=this.#e,r=this.#t.toSyscall(),s=Object.fromEntries(Array.from(this.#r.entries()).map(([n,l])=>[n,m.toSyscall(l)])),a=this.#s.map(n=>m.toSyscall(n));return{hash:e,function:r,env:s,args:a}}static fromSyscall(e){let r=e.hash,s=T.fromSyscall(e.function),a=new Map(Object.entries(e.env).map(([l,y])=>[l,m.fromSyscall(y)])),n=e.args.map(l=>m.fromSyscall(l));return new v({hash:r,function:s,env:a,args:n})}async run(){let e=await R.run(E.toSyscall(this));return m.fromSyscall(e)}};function oe(t,e){return{callSites:e.map(s=>({typeName:s.getTypeName(),functionName:s.getFunctionName(),methodName:s.getMethodName(),fileName:s.getFileName(),lineNumber:s.getLineNumber(),columnNumber:s.getColumnNumber(),isEval:s.isEval(),isNative:s.isNative(),isConstructor:s.isConstructor(),isAsync:s.isAsync(),isPromiseAll:s.isPromiseAll(),promiseIndex:s.getPromiseIndex()}))}}var ie=async t=>{let e=M();return o.fromSyscall(await X(e,t))};var W=(...t)=>{let e=t.map(r=>me(r)).join(" ");Y(e)},me=t=>L(t,new Set),L=(t,e)=>{switch(typeof t){case"string":return`"${t}"`;case"number":return t.toString();case"boolean":return t?"true":"false";case"undefined":return"undefined";case"object":return ye(t,e);case"function":return`[function ${t.name??"(anonymous)"}]`;case"symbol":return"[symbol]";case"bigint":return t.toString()}},ye=(t,e)=>{if(t===null)return"null";if(e.has(t))return"[circular]";if(e.add(t),t instanceof Array)return`[${t.map(r=>L(r,e)).join(", ")}]`;if(t instanceof Error)return t.stack??"";if(t instanceof Promise)return"[promise]";{let r="";t.constructor!==void 0&&t.constructor.name!=="Object"&&(r=`${t.constructor.name} `);let s=Object.entries(t).map(([a,n])=>`${a}: ${L(n,e)}`);return`${r}{ ${s.join(", ")} }`}};var ce=t=>{if(typeof t=="string")return t;{let{arch:e,os:r}=t;return`${e}_${r}`}},q;(r=>(r.arch=s=>{switch(s){case"amd64_linux":case"amd64_macos":return"amd64";case"arm64_linux":case"arm64_macos":return"arm64";default:throw new Error("Invalid system.")}},r.os=s=>{switch(s){case"amd64_linux":case"arm64_linux":return"linux";case"amd64_macos":case"arm64_macos":return"macos";default:throw new Error("Invalid system.")}}))(q||={});Object.defineProperties(Error,{prepareStackTrace:{value:oe}});var pe={log:W};Object.defineProperties(globalThis,{console:{value:pe}});var fe={Artifact:o,Blob:h,Directory:u,File:p,Function:T,Path:c,Placeholder:b,Symlink:f,System:q,Template:i,Value:m,blob:F,call:K,directory:B,download:ae,env:z,file:N,function:se,include:ie,log:W,nullish:_,output:le,path:g,placeholder:$,process:ne,resolve:d,symlink:re,system:ce,template:S};Object.defineProperties(globalThis,{tg:{value:fe},t:{value:U}});})();
//# sourceMappingURL=global.js.map
