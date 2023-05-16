"use strict";(()=>{var b=(a,e)=>{if(!a)throw new Error(e??"Failed assertion.")},k=a=>{throw new Error(a??"Reached unreachable code.")};var B={bundle:async a=>{try{return await syscall("artifact_bundle",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},get:async a=>{try{return await syscall("artifact_get",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},q={decode:a=>{try{return syscall("base64_decode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:a=>{try{return syscall("base64_encode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},_={bytes:async a=>{try{return await syscall("blob_bytes",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},new:async a=>{try{return await syscall("blob_new",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},text:async a=>{try{return await syscall("blob_text",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},G={new:async a=>{try{return await syscall("call_new",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}};var Z={new:async a=>{try{return await syscall("download_new",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},J={new:async a=>{try{return await syscall("directory_new",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},Q={new:async a=>{try{return await syscall("file_new",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},X={decode:a=>{try{return syscall("hex_decode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:a=>{try{return syscall("hex_encode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},Y=async(a,e)=>{try{return await syscall("include",a,e)}catch(t){throw new Error("The syscall failed.",{cause:t})}},ee={decode:a=>{try{return syscall("json_decode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:a=>{try{return syscall("json_encode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},te=a=>{try{return syscall("log",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},R={get:async a=>{try{return await syscall("operation_get",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},run:async a=>{try{return await syscall("operation_run",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},re={new:async a=>{try{return await syscall("process_new",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},$=a=>{try{return syscall("stack_frame",a+1)}catch(e){throw new Error("The syscall failed.",{cause:e})}},ae={new:async a=>{try{return await syscall("symlink_new",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},se={decode:a=>{try{return syscall("toml_decode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:a=>{try{return syscall("toml_encode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},ne={decode:a=>{try{return syscall("utf8_decode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:a=>{try{return syscall("utf8_encode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},le={decode:a=>{try{return syscall("yaml_decode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:a=>{try{return syscall("yaml_encode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}};var f=class{#e;#t;#r;#a;static async new(e){let t=await A(e),s,r,n;if(p.Arg.is(t))s=await F(t),r=!1,n=[];else{if(f.is(t))return t;s=await F(t.blob),r=t.executable??!1,n=t.references??[]}return f.fromSyscall(await Q.new({blob:s.toSyscall(),executable:r,references:n.map(l=>c.toSyscall(l))}))}constructor(e){this.#e=e.hash,this.#t=e.blob,this.#r=e.executable,this.#a=e.references}static is(e){return e instanceof f}toSyscall(){return{hash:this.#e,blob:this.#t.toSyscall(),executable:this.#r,references:this.#a}}static fromSyscall(e){return new f({hash:e.hash,blob:p.fromSyscall(e.blob),executable:e.executable,references:e.references})}hash(){return this.#e}blob(){return this.#t}executable(){return this.#r}async references(){return await Promise.all(this.#a.map(c.get))}async bytes(){return await this.blob().bytes()}async text(){return await this.blob().text()}},N=f.new;var m=class{#e;static new(...e){let t=[],s=n=>{if(typeof n=="string")for(let l of n.split("/"))l===""||l==="."||(l===".."?t.push({kind:"parent"}):t.push({kind:"normal",value:l}));else if(m.Component.is(n))t.push(n);else if(n instanceof m)t.push(...n.components());else if(n instanceof Array)for(let l of n)s(l)};for(let n of e)s(n);let r=new m;for(let n of t)r.push(n);return r}constructor(e=[]){this.#e=e}static is(e){return e instanceof m}toSyscall(){return this.toString()}static fromSyscall(e){return d(e)}components(){return[...this.#e]}push(e){if(e.kind==="parent"){let t=this.#e.at(-1);t===void 0||t.kind==="parent"?this.#e.push(e):this.#e.pop()}else this.#e.push(e)}join(e){let t=d(this);for(let s of d(e).components())t.push(s);return t}diff(e){let t=d(e),s=d(this);for(;;){let n=t.#e.at(0),l=s.#e.at(0);if(n&&l&&m.Component.equal(n,l))t.#e.shift(),s.#e.shift();else break}if(t.#e.at(0)?.kind==="parent")throw new Error(`There is no valid path from "${t}" to "${s}".`);return d(Array.from({length:t.#e.length},()=>({kind:"parent"})),s)}toString(){return this.#e.map(e=>{switch(e.kind){case"parent":return"..";case"normal":return e.value}}).join("/")}};(e=>{let a;(r=>(r.is=n=>typeof n=="object"&&n!==null&&"kind"in n&&(n.kind==="parent"||n.kind==="normal"),r.equal=(n,l)=>n.kind===l.kind&&(n.kind==="normal"&&l.kind==="normal"?n.value===l.value:!0)))(a=e.Component||={})})(m||={});(e=>{let a;(s=>s.is=r=>typeof r=="string"||e.Component.is(r)||r instanceof e||r instanceof Array&&r.every(e.Arg.is))(a=e.Arg||={})})(m||={});var d=m.new;var h=class{#e;static new(e){return new h(e)}constructor(e){this.#e=e}static is(e){return e instanceof h}toSyscall(){return{name:this.#e}}static fromSyscall(e){let t=e.name;return new h(t)}name(){return this.#e}},D=h.new;var H=async(a,...e)=>{let t=[];for(let s=0;s<a.length-1;s++){let r=a[s];t.push(r);let n=e[s];t.push(n)}return t.push(a[a.length-1]),await O(...t)},o=class{#e;static async new(...e){let t=[],s=n=>{if(o.Component.is(n))t.push(n);else if(n instanceof m)t.push(n.toString());else if(n instanceof o)t.push(...n.components());else if(n instanceof Array)for(let l of n)s(l)};for(let n of await Promise.all(e.map(A)))s(n);let r=[];for(let n of t){let l=r.at(-1);n!==""&&(typeof l=="string"&&typeof n=="string"?r.splice(-1,1,l+n):r.push(n))}return t=r,t=ge(t),new o(t)}constructor(e){this.#e=e}static is(e){return e instanceof o}static async join(e,...t){let s=await O(e),r=await Promise.all(t.map(l=>O(l)));r=r.filter(l=>l.components().length>0);let n=[];for(let l=0;l<r.length;l++){l>0&&n.push(s);let u=r[l];b(u),n.push(u)}return O(...n)}toSyscall(){return{components:this.#e.map(t=>o.Component.toSyscall(t))}}static fromSyscall(e){let t=e.components.map(s=>o.Component.fromSyscall(s));return new o(t)}components(){return[...this.#e]}};(e=>{let a;(n=>(n.is=l=>typeof l=="string"||c.is(l)||l instanceof h,n.toSyscall=l=>typeof l=="string"?{kind:"string",value:l}:c.is(l)?{kind:"artifact",value:c.toSyscall(l)}:l instanceof h?{kind:"placeholder",value:l.toSyscall()}:k(),n.fromSyscall=l=>{switch(l.kind){case"string":return l.value;case"artifact":return c.fromSyscall(l.value);case"placeholder":return h.fromSyscall(l.value);default:return k()}}))(a=e.Component||={})})(o||={});(e=>{let a;(s=>s.is=r=>e.Component.is(r)||r instanceof m||r instanceof e||r instanceof Array&&r.every(e.Arg.is))(a=e.Arg||={})})(o||={});var O=o.new,de=a=>{let e=a.split(`
`);if(e.length!=1&&(e=e.filter(t=>!/^\s*$/.exec(t)),e=e.map(t=>/^\s*/.exec(t)?.map(r=>r)??[]).flat(),e.length!=0))return e.reduce((t,s)=>{let r=t?.length??0,n=s?.length??0;return r<n?t:s})},ge=a=>{let e;for(let t of a)if(typeof t=="string"){let s=de(t);(s&&!e||s&&e&&s.length<e.length)&&(e=s)}if(e){let t=e;a=a.map(s=>typeof s=="string"?s.split(`
`).filter(r=>!/^\s*$/.exec(r)).map(r=>(r.startsWith(t)&&(r=r.replace(t,"")),r)).join(`
`):s)}return a};var y=class{#e;#t;static async new(e){let t=await A(e),s,r;if(typeof t=="string")r=t;else if(m.is(t))r=t.toString();else if(c.is(t))s=t;else if(t instanceof o){b(t.components().length<=2);let[l,u]=t.components();if(typeof l=="string"&&u===void 0)r=l;else if(c.is(l)&&u===void 0)s=l;else if(c.is(l)&&typeof u=="string")s=l,b(u.startsWith("/")),r=u.slice(1);else throw new Error("Invalid template.")}else{if(t instanceof y)return t;if(typeof t=="object"){s=t.artifact;let l=t.path;typeof l=="string"?r=l:m.is(l)&&(r=l.toString())}}let n;return s!==void 0&&r!==void 0?n=await H`${s}/${r}`:s!==void 0&&r===void 0?n=await H`${s}`:s===void 0&&r!==void 0?n=await H`${r}`:n=await H``,y.fromSyscall(await ae.new({target:n.toSyscall()}))}constructor(e){this.#e=e.hash,this.#t=e.target}static is(e){return e instanceof y}toSyscall(){let e=this.#e,t=this.#t.toSyscall();return{hash:e,target:t}}static fromSyscall(e){let t=e.hash,s=o.fromSyscall(e.target);return new y({hash:t,target:s})}hash(){return this.#e}target(){return this.#t}artifact(){let e=this.#t.components().at(0);if(c.is(e))return e}path(){let[e,t]=this.#t.components();if(typeof e=="string"&&t===void 0)return d(e);if(c.is(e)&&t===void 0)return d();if(c.is(e)&&typeof t=="string")return d(t);throw new Error("Invalid template.")}async resolve(){let e=this;for(;y.is(e);){let t=e.artifact(),s=e.path();if(i.is(t))e=await t.get(s);else if(f.is(t))b(s.components().length===0),e=t;else if(y.is(t))b(s.components().length===0),e=t;else throw new Error("Cannot resolve a symlink without an artifact in its target.")}return e}},oe=y.new;var A=async a=>{if(a=await a,a===void 0||typeof a=="boolean"||typeof a=="number"||typeof a=="string"||a instanceof Uint8Array||a instanceof m||a instanceof p||a instanceof i||a instanceof f||a instanceof y||a instanceof h||a instanceof o)return a;if(a instanceof Array)return await Promise.all(a.map(e=>A(e)));if(typeof a=="object")return Object.fromEntries(await Promise.all(Object.entries(a).map(async([e,t])=>[e,await A(t)])));throw new Error("Invalid value to resolve.")};var p=class{#e;static async new(e){let t=await A(e),s;if(t instanceof Uint8Array||typeof t=="string")s=t;else return t;return p.fromSyscall(await _.new(s))}constructor(e){this.#e=e.hash}static is(e){return e instanceof p}toSyscall(){return{hash:this.#e}}static fromSyscall(e){let t=e.hash;return new p({hash:t})}hash(){return this.#e}async bytes(){return await _.bytes(this.toSyscall())}async text(){return await _.text(this.toSyscall())}};(e=>{let a;(s=>s.is=r=>r instanceof Uint8Array||typeof r=="string"||r instanceof e)(a=e.Arg||={})})(p||={});var F=p.new;var i=class{#e;#t;static async new(...e){let t=new Map;for(let s of await Promise.all(e.map(A)))if(s!==void 0){if(s instanceof i)for(let[r,n]of await s.entries()){let l=t.get(r);l instanceof i&&n instanceof i&&(n=await i.new(l,n)),t.set(r,n)}else if(typeof s=="object")for(let[r,n]of Object.entries(s)){let[l,...u]=d(r).components();if(l===void 0)throw new Error("The path must have at least one component.");if(l.kind!=="normal")throw new Error("Invalid path component.");let x=l.value,v=t.get(x);if(v instanceof i||(v=void 0),u.length>0){let w=d(u).toString(),j=await i.new(v,{[w]:n});t.set(x,j)}else if(n===void 0)t.delete(x);else if(p.Arg.is(n)){let w=await N(n);t.set(x,w)}else if(f.is(n)||y.is(n))t.set(x,n);else{let w=await i.new(v,n);t.set(x,w)}}}return i.fromSyscall(await J.new({entries:Object.fromEntries(Array.from(t,([s,r])=>[s,c.toSyscall(r)]))}))}constructor(e){this.#e=e.hash,this.#t=e.entries}static is(e){return e instanceof i}toSyscall(){return{hash:this.#e,entries:Object.fromEntries(this.#t)}}static fromSyscall(e){let t=e.hash,s=new Map(Object.entries(e.entries));return new i({hash:t,entries:s})}hash(){return this.#e}async get(e){let t=await this.tryGet(e);return b(t,`Failed to get the directory entry "${e}".`),t}async tryGet(e){let t=this;for(let s of d(e).components()){if(b(s.kind==="normal"),!(t instanceof i))return;let r=t.#t.get(s.value);if(!r)return;t=await c.get(r)}return t}async entries(){let e=new Map;for await(let[t,s]of this)e.set(t,s);return e}async bundle(){let e=c.fromSyscall(await B.bundle(c.toSyscall(this)));return b(i.is(e)),e}async*walk(){for await(let[e,t]of this)if(yield[d(e),t],i.is(t))for await(let[s,r]of t.walk())yield[d(e).join(s),r]}*[Symbol.iterator](){for(let[e,t]of this.#t)yield[e,t]}async*[Symbol.asyncIterator](){for(let e of this.#t.keys())yield[e,await this.get(e)]}},ie=i.new;var c;(r=>(r.is=n=>n instanceof i||n instanceof f||n instanceof y,r.get=async n=>r.fromSyscall(await B.get(n)),r.toSyscall=n=>n instanceof i?{kind:"directory",value:n.toSyscall()}:n instanceof f?{kind:"file",value:n.toSyscall()}:n instanceof y?{kind:"symlink",value:n.toSyscall()}:k(),r.fromSyscall=n=>{switch(n.kind){case"directory":return i.fromSyscall(n.value);case"file":return f.fromSyscall(n.value);case"symlink":return y.fromSyscall(n.value);default:return k()}}))(c||={});var g;(s=>(s.is=r=>r===void 0||typeof r=="boolean"||typeof r=="number"||typeof r=="string"||r instanceof Uint8Array||r instanceof m||r instanceof p||r instanceof i||r instanceof f||r instanceof y||r instanceof h||r instanceof o||r instanceof Array||typeof r=="object",s.toSyscall=r=>r===void 0?{kind:"null",value:null}:typeof r=="boolean"?{kind:"bool",value:r}:typeof r=="number"?{kind:"number",value:r}:typeof r=="string"?{kind:"string",value:r}:r instanceof Uint8Array?{kind:"bytes",value:r}:r instanceof m?{kind:"path",value:r.toSyscall()}:r instanceof p?{kind:"blob",value:r.toSyscall()}:c.is(r)?{kind:"artifact",value:c.toSyscall(r)}:r instanceof h?{kind:"placeholder",value:r.toSyscall()}:r instanceof o?{kind:"template",value:r.toSyscall()}:r instanceof Array?{kind:"array",value:r.map(l=>s.toSyscall(l))}:typeof r=="object"?{kind:"object",value:Object.fromEntries(Object.entries(r).map(([l,u])=>[l,s.toSyscall(u)]))}:k(),s.fromSyscall=r=>{switch(r.kind){case"null":return;case"bool":return r.value;case"number":return r.value;case"string":return r.value;case"bytes":return r.value;case"path":return m.fromSyscall(r.value);case"blob":return p.fromSyscall(r.value);case"artifact":return c.fromSyscall(r.value);case"placeholder":return h.fromSyscall(r.value);case"template":return o.fromSyscall(r.value);case"array":return r.value.map(n=>s.fromSyscall(n));case"object":return Object.fromEntries(Object.entries(r.value).map(([n,l])=>[n,s.fromSyscall(l)]));default:return k()}}))(g||={});var S=class extends globalThis.Function{packageInstanceHash;modulePath;name;f;static new(e){let{module:t,line:s}=$(1);b(t.kind==="normal");let r=t.value.packageInstanceHash,n=t.value.modulePath,l;if(s.startsWith("export default "))l="default";else if(s.startsWith("export let ")){let u=s.match(/^export let ([a-zA-Z0-9]+)\b/)?.at(1);if(!u)throw new Error("Invalid use of tg.function.");l=u}else throw new Error("Invalid use of tg.function.");return new S({packageInstanceHash:r,modulePath:n,name:l,f:e})}constructor(e){return super(),this.packageInstanceHash=e.packageInstanceHash,this.modulePath=d(e.modulePath),this.name=e.name,this.f=e.f,new Proxy(this,{apply:async(t,s,r)=>{let n=await Promise.all(r.map(A));return await M({function:t,args:n})}})}static is(e){return e instanceof S}toSyscall(){let e=this.packageInstanceHash,t=this.modulePath.toString(),s=this.name;return{packageInstanceHash:e,modulePath:t,name:s}}static fromSyscall(e){let t=e.packageInstanceHash,s=e.modulePath,r=e.name;return new S({packageInstanceHash:t,modulePath:s,name:r})}async run(e,t){I.value=Object.fromEntries(Object.entries(e).map(([n,l])=>[n,g.fromSyscall(l)]));let s=t.map(g.fromSyscall);b(this.f);let r=await this.f(...s);return g.toSyscall(r)}},ce=S.new;var me=async a=>await(await T.new(a)).run(),T=class{#e;#t;#r;#a;#s;static async new(e){return T.fromSyscall(await Z.new({url:e.url,unpack:e.unpack??!1,checksum:e.checksum??void 0,unsafe:e.unsafe??!1}))}constructor(e){this.#e=e.hash,this.#t=e.url,this.#r=e.unpack??!1,this.#a=e.checksum??void 0,this.#s=e.unsafe??!1}static is(e){return e instanceof T}hash(){return this.#e}toSyscall(){return{hash:this.#e,url:this.#t,unpack:this.#r,checksum:this.#a,unsafe:this.#s}}static fromSyscall(e){return new T({hash:e.hash,url:e.url,unpack:e.unpack,checksum:e.checksum,unsafe:e.unsafe})}async run(){let e=await R.run(U.toSyscall(this));return g.fromSyscall(e)}};var ue=async a=>await(await V.new(a)).run(),fe=D("output"),V=class{#e;#t;#r;#a;#s;#n;#l;#o;#i;static async new(e){let t=await A(e),s=t.system,r=await O(t.executable),n=Object.fromEntries(await Promise.all(Object.entries(t.env??{}).map(async([E,W])=>[E,await O(W)]))),l=Object.fromEntries(Object.entries(n).map(([E,W])=>[E,W.toSyscall()])),u=await Promise.all((t.args??[]).map(async E=>(await O(E)).toSyscall())),x=t.checksum??void 0,v=t.unsafe??!1,w=t.network??!1,j=t.hostPaths??[];return V.fromSyscall(await re.new({system:s,executable:r.toSyscall(),env:l,args:u,checksum:x,unsafe:v,network:w,hostPaths:j}))}constructor(e){this.#e=e.hash,this.#t=e.system,this.#r=e.executable,this.#a=e.env,this.#s=e.args,this.#n=e.checksum,this.#l=e.unsafe,this.#o=e.network,this.#i=e.hostPaths}hash(){return this.#e}toSyscall(){let e=this.#e,t=this.#t,s=this.#r.toSyscall(),r=Object.fromEntries(Object.entries(this.#a).map(([w,j])=>[w,j.toSyscall()])),n=this.#s.map(w=>w.toSyscall()),l=this.#n,u=this.#l,x=this.#o,v=this.#i;return{hash:e,system:t,executable:s,env:r,args:n,checksum:l,unsafe:u,network:x,hostPaths:v}}static fromSyscall(e){let t=e.hash,s=e.system,r=o.fromSyscall(e.executable),n=Object.fromEntries(Object.entries(e.env).map(([j,E])=>[j,o.fromSyscall(E)])),l=e.args.map(j=>o.fromSyscall(j)),u=e.checksum,x=e.unsafe,v=e.network,w=e.hostPaths;return new V({hash:t,system:s,executable:r,env:n,args:l,checksum:u,unsafe:x,network:v,hostPaths:w})}async run(){let e=await R.run(U.toSyscall(this));return g.fromSyscall(e)}};var U;(s=>(s.is=r=>r instanceof C||r instanceof T||r instanceof V,s.toSyscall=r=>r instanceof T?{kind:"download",value:r.toSyscall()}:r instanceof V?{kind:"process",value:r.toSyscall()}:r instanceof C?{kind:"call",value:r.toSyscall()}:k(),s.fromSyscall=(r,n)=>{switch(n.kind){case"download":return T.fromSyscall(n.value);case"process":return V.fromSyscall(n.value);case"call":return C.fromSyscall(n.value);default:return k()}}))(U||={});var I={get(){return b(this.value),this.value}},M=async a=>await(await C.new(a)).run(),C=class{#e;#t;#r;#a;static async new(e){let t=e.function.toSyscall(),s=Object.fromEntries(Object.entries(e.env??I.get()).map(([l,u])=>[l,g.toSyscall(u)])),r=(e.args??[]).map(l=>g.toSyscall(l));return C.fromSyscall(await G.new({function:t,env:s,args:r}))}constructor(e){this.#e=e.hash,this.#t=e.function,this.#r=e.env,this.#a=e.args}static is(e){return e instanceof C}hash(){return this.#e}toSyscall(){let e=this.#e,t=this.#t.toSyscall(),s=Object.fromEntries(Object.entries(this.#r).map(([n,l])=>[n,g.toSyscall(l)])),r=this.#a.map(n=>g.toSyscall(n));return{hash:e,function:t,env:s,args:r}}static fromSyscall(e){let t=e.hash,s=S.fromSyscall(e.function),r=Object.fromEntries(Object.entries(e.env).map(([l,u])=>[l,g.fromSyscall(u)])),n=e.args.map(l=>g.fromSyscall(l));return new C({hash:t,function:s,env:r,args:n})}async run(){let e=await R.run(U.toSyscall(this));return g.fromSyscall(e)}};var ye=(a,e)=>({callSites:e.map(s=>({typeName:s.getTypeName(),functionName:s.getFunctionName(),methodName:s.getMethodName(),fileName:s.getFileName(),lineNumber:s.getLineNumber(),columnNumber:s.getColumnNumber(),isEval:s.isEval(),isNative:s.isNative(),isConstructor:s.isConstructor(),isAsync:s.isAsync(),isPromiseAll:s.isPromiseAll(),promiseIndex:s.getPromiseIndex()}))});var pe=async a=>{let e=$(1);return c.fromSyscall(await Y(e,a))};var K=(...a)=>{let e=a.map(t=>Ae(t)).join(" ");te(e)},Ae=a=>z(a,new WeakSet),z=(a,e)=>{switch(typeof a){case"string":return`"${a}"`;case"number":return a.toString();case"boolean":return a?"true":"false";case"undefined":return"undefined";case"object":return a===null?"null":be(a,e);case"function":return`(function "${a.name??"(anonymous)"}")`;case"symbol":return"(symbol)";case"bigint":return a.toString()}},be=(a,e)=>{if(e.has(a))return"(circular)";if(e.add(a),a instanceof Array)return`[${a.map(t=>z(t,e)).join(", ")}]`;if(a instanceof Error)return a.stack??"";if(a instanceof Promise)return"(promise)";if(a instanceof i)return`(tg.directory ${a.hash()})`;if(a instanceof f)return`(tg.file ${a.hash()})`;if(a instanceof y)return`(tg.symlink ${a.hash()})`;if(a instanceof h)return`(tg.placeholder "${a.name()}")`;if(a instanceof o)return`(tg.template "${a.components().map(s=>typeof s=="string"?s:`\${${z(s,e)}}`).join("")}")`;{let t="";a.constructor!==void 0&&a.constructor.name!=="Object"&&(t=`${a.constructor.name} `);let s=Object.entries(a).map(([r,n])=>`${r}: ${z(n,e)}`);return`${t}{ ${s.join(", ")} }`}};var he=a=>{if(typeof a=="string")return a;{let{arch:e,os:t}=a;return`${e}_${t}`}},L;(s=>(s.is=r=>r==="amd64_linux"||r==="arm64_linux"||r==="amd64_macos"||r==="arm64_macos",s.arch=r=>{switch(r){case"amd64_linux":case"amd64_macos":return"amd64";case"arm64_linux":case"arm64_macos":return"arm64";default:throw new Error("Invalid system.")}},s.os=r=>{switch(r){case"amd64_linux":case"arm64_linux":return"linux";case"amd64_macos":case"arm64_macos":return"macos";default:throw new Error("Invalid system.")}}))(L||={});Object.defineProperties(Error,{prepareStackTrace:{value:ye}});var we={log:K};Object.defineProperties(globalThis,{console:{value:we}});var xe={Artifact:c,Blob:p,Directory:i,File:f,Function:S,Path:m,Placeholder:h,Symlink:y,System:L,Template:o,Value:g,base64:q,blob:F,call:M,directory:ie,download:me,env:I,file:N,function:ce,hex:X,include:pe,json:ee,log:K,output:fe,path:d,placeholder:D,process:ue,resolve:A,symlink:oe,system:he,template:O,toml:se,utf8:ne,yaml:le};Object.defineProperties(globalThis,{tg:{value:xe},t:{value:H}});})();
//# sourceMappingURL=global.js.map
