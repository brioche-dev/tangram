"use strict";(()=>{var b=(a,e)=>{if(!a)throw new Error(e??"Failed assertion.")},k=a=>{throw new Error(a??"Reached unreachable code.")};var B={bundle:async a=>{try{return await syscall("artifact_bundle",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},get:async a=>{try{return await syscall("artifact_get",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},q={decode:a=>{try{return syscall("base64_decode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:a=>{try{return syscall("base64_encode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},_={bytes:async a=>{try{return await syscall("blob_bytes",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},new:async a=>{try{return await syscall("blob_new",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},text:async a=>{try{return await syscall("blob_text",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},G={new:async a=>{try{return await syscall("call_new",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}};var Z={new:async a=>{try{return await syscall("download_new",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},J={new:async a=>{try{return await syscall("directory_new",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},Q={new:async a=>{try{return await syscall("file_new",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},X={decode:a=>{try{return syscall("hex_decode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:a=>{try{return syscall("hex_encode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},Y=async(a,e)=>{try{return await syscall("include",a,e)}catch(t){throw new Error("The syscall failed.",{cause:t})}},ee={decode:a=>{try{return syscall("json_decode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:a=>{try{return syscall("json_encode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},te=a=>{try{return syscall("log",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},R={get:async a=>{try{return await syscall("operation_get",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},run:async a=>{try{return await syscall("operation_run",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},re={new:async a=>{try{return await syscall("process_new",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},N=a=>{try{return syscall("stack_frame",a+1)}catch(e){throw new Error("The syscall failed.",{cause:e})}},ae={new:async a=>{try{return await syscall("symlink_new",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},se={decode:a=>{try{return syscall("toml_decode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:a=>{try{return syscall("toml_encode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},ne={decode:a=>{try{return syscall("utf8_decode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:a=>{try{return syscall("utf8_encode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},oe={decode:a=>{try{return syscall("yaml_decode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:a=>{try{return syscall("yaml_encode",a)}catch(e){throw new Error("The syscall failed.",{cause:e})}}};var y=class{#e;#t;#r;#a;static async new(e){let t=await A(e),n,r,s;if(f.Arg.is(t))n=await F(t),r=!1,s=[];else{if(y.is(t))return t;n=await F(t.blob),r=t.executable??!1,s=t.references??[]}return y.fromSyscall(await Q.new({blob:n.toSyscall(),executable:r,references:s.map(o=>i.toSyscall(o))}))}constructor(e){this.#e=e.hash,this.#t=e.blob,this.#r=e.executable,this.#a=e.references}static is(e){return e instanceof y}toSyscall(){return{hash:this.#e,blob:this.#t.toSyscall(),executable:this.#r,references:this.#a}}static fromSyscall(e){return new y({hash:e.hash,blob:f.fromSyscall(e.blob),executable:e.executable,references:e.references})}hash(){return this.#e}blob(){return this.#t}executable(){return this.#r}async references(){return await Promise.all(this.#a.map(i.get))}async bytes(){return await this.blob().bytes()}async text(){return await this.blob().text()}},D=y.new;var m=class{#e;static new(...e){let t=[],n=s=>{if(typeof s=="string")for(let o of s.split("/"))o===""||o==="."||(o===".."?t.push({kind:"parent"}):t.push({kind:"normal",value:o}));else if(m.Component.is(s))t.push(s);else if(s instanceof m)t.push(...s.components());else if(s instanceof Array)for(let o of s)n(o)};for(let s of e)n(s);let r=new m;for(let s of t)r.push(s);return r}constructor(e=[]){this.#e=e}static is(e){return e instanceof m}toSyscall(){return this.toString()}static fromSyscall(e){return h(e)}components(){return[...this.#e]}push(e){if(e.kind==="parent"){let t=this.#e.at(-1);t===void 0||t.kind==="parent"?this.#e.push(e):this.#e.pop()}else this.#e.push(e)}join(e){let t=h(this);for(let n of h(e).components())t.push(n);return t}diff(e){let t=h(e),n=h(this);for(;;){let s=t.#e.at(0),o=n.#e.at(0);if(s&&o&&m.Component.equal(s,o))t.#e.shift(),n.#e.shift();else break}if(t.#e.at(0)?.kind==="parent")throw new Error(`There is no valid path from "${t}" to "${n}".`);return h(Array.from({length:t.#e.length},()=>({kind:"parent"})),n)}toString(){return this.#e.map(e=>{switch(e.kind){case"parent":return"..";case"normal":return e.value}}).join("/")}};(e=>{let a;(r=>(r.is=s=>typeof s=="object"&&s!==null&&"kind"in s&&(s.kind==="parent"||s.kind==="normal"),r.equal=(s,o)=>s.kind===o.kind&&(s.kind==="normal"&&o.kind==="normal"?s.value===o.value:!0)))(a=e.Component||={})})(m||={});(e=>{let a;(n=>n.is=r=>typeof r=="string"||e.Component.is(r)||r instanceof e||r instanceof Array&&r.every(e.Arg.is))(a=e.Arg||={})})(m||={});var h=m.new;var d=class{#e;static new(e){return new d(e)}constructor(e){this.#e=e}static is(e){return e instanceof d}toSyscall(){return{name:this.#e}}static fromSyscall(e){let t=e.name;return new d(t)}name(){return this.#e}},M=d.new;var H=async(a,...e)=>{let t=[];for(let n=0;n<a.length-1;n++){let r=a[n];t.push(r);let s=e[n];t.push(s)}return t.push(a[a.length-1]),await O(...t)},l=class{#e;static async new(...e){let t=[],n=s=>{if(l.Component.is(s))t.push(s);else if(s instanceof m)t.push(s.toString());else if(s instanceof l)t.push(...s.components());else if(s instanceof Array)for(let o of s)n(o)};for(let s of await Promise.all(e.map(A)))n(s);let r=[];for(let s of t){let o=r.at(-1);s!==""&&(typeof o=="string"&&typeof s=="string"?r.splice(-1,1,o+s):r.push(s))}return t=r,new l(t)}constructor(e){this.#e=e}static is(e){return e instanceof l}static async join(e,...t){let n=await O(e),r=await Promise.all(t.map(o=>O(o)));r=r.filter(o=>o.components().length>0);let s=[];for(let o=0;o<r.length;o++){o>0&&s.push(n);let u=r[o];b(u),s.push(u)}return O(...s)}toSyscall(){return{components:this.#e.map(t=>l.Component.toSyscall(t))}}static fromSyscall(e){let t=e.components.map(n=>l.Component.fromSyscall(n));return new l(t)}components(){return[...this.#e]}};(e=>{let a;(s=>(s.is=o=>typeof o=="string"||i.is(o)||o instanceof d,s.toSyscall=o=>typeof o=="string"?{kind:"string",value:o}:i.is(o)?{kind:"artifact",value:i.toSyscall(o)}:o instanceof d?{kind:"placeholder",value:o.toSyscall()}:k(),s.fromSyscall=o=>{switch(o.kind){case"string":return o.value;case"artifact":return i.fromSyscall(o.value);case"placeholder":return d.fromSyscall(o.value);default:return k()}}))(a=e.Component||={})})(l||={});(e=>{let a;(n=>n.is=r=>e.Component.is(r)||r instanceof m||r instanceof e||r instanceof Array&&r.every(e.Arg.is))(a=e.Arg||={})})(l||={});var O=l.new;var p=class{#e;#t;static async new(e){let t=await A(e),n,r;if(typeof t=="string")r=t;else if(m.is(t))r=t.toString();else if(i.is(t))n=t;else if(t instanceof l){b(t.components().length<=2);let[o,u]=t.components();if(typeof o=="string"&&u===void 0)r=o;else if(i.is(o)&&u===void 0)n=o;else if(i.is(o)&&typeof u=="string")n=o,b(u.startsWith("/")),r=u.slice(1);else throw new Error("Invalid template.")}else{if(t instanceof p)return t;if(typeof t=="object"){n=t.artifact;let o=t.path;typeof o=="string"?r=o:m.is(o)&&(r=o.toString())}}let s;return n!==void 0&&r!==void 0?s=await H`${n}/${r}`:n!==void 0&&r===void 0?s=await H`${n}`:n===void 0&&r!==void 0?s=await H`${r}`:s=await H``,p.fromSyscall(await ae.new({target:s.toSyscall()}))}constructor(e){this.#e=e.hash,this.#t=e.target}static is(e){return e instanceof p}toSyscall(){let e=this.#e,t=this.#t.toSyscall();return{hash:e,target:t}}static fromSyscall(e){let t=e.hash,n=l.fromSyscall(e.target);return new p({hash:t,target:n})}hash(){return this.#e}target(){return this.#t}artifact(){let e=this.#t.components().at(0);if(i.is(e))return e}path(){let[e,t]=this.#t.components();if(typeof e=="string"&&t===void 0)return h(e);if(i.is(e)&&t===void 0)return h();if(i.is(e)&&typeof t=="string")return h(t);throw new Error("Invalid template.")}async resolve(){let e=this;for(;p.is(e);){let t=e.artifact(),n=e.path();if(c.is(t))e=await t.get(n);else if(y.is(t))b(n.components().length===0),e=t;else if(p.is(t))b(n.components().length===0),e=t;else throw new Error("Cannot resolve a symlink without an artifact in its target.")}return e}},le=p.new;var A=async a=>{if(a=await a,a===void 0||typeof a=="boolean"||typeof a=="number"||typeof a=="string"||a instanceof Uint8Array||a instanceof m||a instanceof f||a instanceof c||a instanceof y||a instanceof p||a instanceof d||a instanceof l)return a;if(a instanceof Array)return await Promise.all(a.map(e=>A(e)));if(typeof a=="object")return Object.fromEntries(await Promise.all(Object.entries(a).map(async([e,t])=>[e,await A(t)])));throw new Error("Invalid value to resolve.")};var f=class{#e;static async new(e){let t=await A(e),n;if(t instanceof Uint8Array||typeof t=="string")n=t;else return t;return f.fromSyscall(await _.new(n))}constructor(e){this.#e=e.hash}static is(e){return e instanceof f}toSyscall(){return{hash:this.#e}}static fromSyscall(e){let t=e.hash;return new f({hash:t})}hash(){return this.#e}async bytes(){return await _.bytes(this.toSyscall())}async text(){return await _.text(this.toSyscall())}};(e=>{let a;(n=>n.is=r=>r instanceof Uint8Array||typeof r=="string"||r instanceof e)(a=e.Arg||={})})(f||={});var F=f.new;var c=class{#e;#t;static async new(...e){let t=new Map;for(let n of await Promise.all(e.map(A)))if(n!==void 0){if(n instanceof c)for(let[r,s]of await n.entries()){let o=t.get(r);o instanceof c&&s instanceof c&&(s=await c.new(o,s)),t.set(r,s)}else if(typeof n=="object")for(let[r,s]of Object.entries(n)){let[o,...u]=h(r).components();if(o===void 0)throw new Error("The path must have at least one component.");if(o.kind!=="normal")throw new Error("Invalid path component.");let x=o.value,C=t.get(x);if(C instanceof c||(C=void 0),u.length>0){let w=h(u).toString(),j=await c.new(C,{[w]:s});t.set(x,j)}else if(s===void 0)t.delete(x);else if(f.Arg.is(s)){let w=await D(s);t.set(x,w)}else if(y.is(s)||p.is(s))t.set(x,s);else{let w=await c.new(C,s);t.set(x,w)}}}return c.fromSyscall(await J.new({entries:Object.fromEntries(Array.from(t,([n,r])=>[n,i.toSyscall(r)]))}))}constructor(e){this.#e=e.hash,this.#t=e.entries}static is(e){return e instanceof c}toSyscall(){return{hash:this.#e,entries:Object.fromEntries(this.#t)}}static fromSyscall(e){let t=e.hash,n=new Map(Object.entries(e.entries));return new c({hash:t,entries:n})}hash(){return this.#e}async get(e){let t=await this.tryGet(e);return b(t,`Failed to get the directory entry "${e}".`),t}async tryGet(e){let t=this;for(let n of h(e).components()){if(b(n.kind==="normal"),!(t instanceof c))return;let r=t.#t.get(n.value);if(!r)return;t=await i.get(r)}return t}async entries(){let e=new Map;for await(let[t,n]of this)e.set(t,n);return e}async bundle(){let e=i.fromSyscall(await B.bundle(i.toSyscall(this)));return b(c.is(e)),e}async*walk(){for await(let[e,t]of this)if(yield[h(e),t],c.is(t))for await(let[n,r]of t.walk())yield[h(e).join(n),r]}*[Symbol.iterator](){for(let[e,t]of this.#t)yield[e,t]}async*[Symbol.asyncIterator](){for(let e of this.#t.keys())yield[e,await this.get(e)]}},ie=c.new;var i;(r=>(r.is=s=>s instanceof c||s instanceof y||s instanceof p,r.get=async s=>r.fromSyscall(await B.get(s)),r.toSyscall=s=>s instanceof c?{kind:"directory",value:s.toSyscall()}:s instanceof y?{kind:"file",value:s.toSyscall()}:s instanceof p?{kind:"symlink",value:s.toSyscall()}:k(),r.fromSyscall=s=>{switch(s.kind){case"directory":return c.fromSyscall(s.value);case"file":return y.fromSyscall(s.value);case"symlink":return p.fromSyscall(s.value);default:return k()}}))(i||={});var g;(n=>(n.is=r=>r===void 0||typeof r=="boolean"||typeof r=="number"||typeof r=="string"||r instanceof Uint8Array||r instanceof m||r instanceof f||r instanceof c||r instanceof y||r instanceof p||r instanceof d||r instanceof l||r instanceof Array||typeof r=="object",n.toSyscall=r=>r===void 0?{kind:"null"}:typeof r=="boolean"?{kind:"bool",value:r}:typeof r=="number"?{kind:"number",value:r}:typeof r=="string"?{kind:"string",value:r}:r instanceof Uint8Array?{kind:"bytes",value:r}:r instanceof m?{kind:"path",value:r.toSyscall()}:r instanceof f?{kind:"blob",value:r.toSyscall()}:i.is(r)?{kind:"artifact",value:i.toSyscall(r)}:r instanceof d?{kind:"placeholder",value:r.toSyscall()}:r instanceof l?{kind:"template",value:r.toSyscall()}:r instanceof Array?{kind:"array",value:r.map(o=>n.toSyscall(o))}:typeof r=="object"?{kind:"object",value:Object.fromEntries(Object.entries(r).map(([o,u])=>[o,n.toSyscall(u)]))}:k(),n.fromSyscall=r=>{switch(r.kind){case"null":return;case"bool":return r.value;case"number":return r.value;case"string":return r.value;case"bytes":return r.value;case"path":return m.fromSyscall(r.value);case"blob":return f.fromSyscall(r.value);case"artifact":return i.fromSyscall(r.value);case"placeholder":return d.fromSyscall(r.value);case"template":return l.fromSyscall(r.value);case"array":return r.value.map(s=>n.fromSyscall(s));case"object":return Object.fromEntries(Object.entries(r.value).map(([s,o])=>[s,n.fromSyscall(o)]));default:return k()}}))(g||={});var S=class extends globalThis.Function{packageInstanceHash;modulePath;name;f;static new(e){let{module:t,line:n}=N(1);b(t.kind==="normal");let r=t.value.packageInstanceHash,s=t.value.modulePath,o;if(n.startsWith("export default "))o="default";else if(n.startsWith("export let ")){let u=n.match(/^export let ([a-zA-Z0-9]+)\b/)?.at(1);if(!u)throw new Error("Invalid use of tg.function.");o=u}else throw new Error("Invalid use of tg.function.");return new S({packageInstanceHash:r,modulePath:s,name:o,f:e})}constructor(e){return super(),this.packageInstanceHash=e.packageInstanceHash,this.modulePath=h(e.modulePath),this.name=e.name,this.f=e.f,new Proxy(this,{apply:async(t,n,r)=>{let s=await Promise.all(r.map(A));return await $({function:t,args:s})}})}static is(e){return e instanceof S}toSyscall(){let e=this.packageInstanceHash,t=this.modulePath.toString(),n=this.name;return{packageInstanceHash:e,modulePath:t,name:n}}static fromSyscall(e){let t=e.packageInstanceHash,n=e.modulePath,r=e.name;return new S({packageInstanceHash:t,modulePath:n,name:r})}async run(e,t){I.value=Object.fromEntries(Object.entries(e).map(([s,o])=>[s,g.fromSyscall(o)]));let n=t.map(g.fromSyscall);b(this.f);let r=await this.f(...n);return g.toSyscall(r)}},ce=S.new;var me=async a=>await(await T.new(a)).run(),T=class{#e;#t;#r;#a;#s;static async new(e){return T.fromSyscall(await Z.new({url:e.url,unpack:e.unpack??!1,checksum:e.checksum??void 0,unsafe:e.unsafe??!1}))}constructor(e){this.#e=e.hash,this.#t=e.url,this.#r=e.unpack??!1,this.#a=e.checksum??void 0,this.#s=e.unsafe??!1}static is(e){return e instanceof T}hash(){return this.#e}toSyscall(){return{hash:this.#e,url:this.#t,unpack:this.#r,checksum:this.#a,unsafe:this.#s}}static fromSyscall(e){return new T({hash:e.hash,url:e.url,unpack:e.unpack,checksum:e.checksum,unsafe:e.unsafe})}async run(){let e=await R.run(U.toSyscall(this));return g.fromSyscall(e)}};var ue=async a=>await(await V.new(a)).run(),ye=M("output"),V=class{#e;#t;#r;#a;#s;#n;#o;#l;#i;static async new(e){let t=await A(e),n=t.system,r=await O(t.executable),s=Object.fromEntries(await Promise.all(Object.entries(t.env??{}).map(async([E,z])=>[E,await O(z)]))),o=Object.fromEntries(Object.entries(s).map(([E,z])=>[E,z.toSyscall()])),u=await Promise.all((t.args??[]).map(async E=>(await O(E)).toSyscall())),x=t.checksum??void 0,C=t.unsafe??!1,w=t.network??!1,j=t.hostPaths??[];return V.fromSyscall(await re.new({system:n,executable:r.toSyscall(),env:o,args:u,checksum:x,unsafe:C,network:w,hostPaths:j}))}constructor(e){this.#e=e.hash,this.#t=e.system,this.#r=e.executable,this.#a=e.env,this.#s=e.args,this.#n=e.checksum,this.#o=e.unsafe,this.#l=e.network,this.#i=e.hostPaths}hash(){return this.#e}toSyscall(){let e=this.#e,t=this.#t,n=this.#r.toSyscall(),r=Object.fromEntries(Object.entries(this.#a).map(([w,j])=>[w,j.toSyscall()])),s=this.#s.map(w=>w.toSyscall()),o=this.#n,u=this.#o,x=this.#l,C=this.#i;return{hash:e,system:t,executable:n,env:r,args:s,checksum:o,unsafe:u,network:x,hostPaths:C}}static fromSyscall(e){let t=e.hash,n=e.system,r=l.fromSyscall(e.executable),s=Object.fromEntries(Object.entries(e.env).map(([j,E])=>[j,l.fromSyscall(E)])),o=e.args.map(j=>l.fromSyscall(j)),u=e.checksum,x=e.unsafe,C=e.network,w=e.hostPaths;return new V({hash:t,system:n,executable:r,env:s,args:o,checksum:u,unsafe:x,network:C,hostPaths:w})}async run(){let e=await R.run(U.toSyscall(this));return g.fromSyscall(e)}};var U;(n=>(n.is=r=>r instanceof v||r instanceof T||r instanceof V,n.toSyscall=r=>r instanceof T?{kind:"download",value:r.toSyscall()}:r instanceof V?{kind:"process",value:r.toSyscall()}:r instanceof v?{kind:"call",value:r.toSyscall()}:k(),n.fromSyscall=(r,s)=>{switch(s.kind){case"download":return T.fromSyscall(s.value);case"process":return V.fromSyscall(s.value);case"call":return v.fromSyscall(s.value);default:return k()}}))(U||={});var I={get(){return b(this.value),this.value}},$=async a=>await(await v.new(a)).run(),v=class{#e;#t;#r;#a;static async new(e){let t=e.function.toSyscall(),n=Object.fromEntries(Object.entries(e.env??I.get()).map(([o,u])=>[o,g.toSyscall(u)])),r=(e.args??[]).map(o=>g.toSyscall(o));return v.fromSyscall(await G.new({function:t,env:n,args:r}))}constructor(e){this.#e=e.hash,this.#t=e.function,this.#r=e.env,this.#a=e.args}static is(e){return e instanceof v}hash(){return this.#e}toSyscall(){let e=this.#e,t=this.#t.toSyscall(),n=Object.fromEntries(Object.entries(this.#r).map(([s,o])=>[s,g.toSyscall(o)])),r=this.#a.map(s=>g.toSyscall(s));return{hash:e,function:t,env:n,args:r}}static fromSyscall(e){let t=e.hash,n=S.fromSyscall(e.function),r=Object.fromEntries(Object.entries(e.env).map(([o,u])=>[o,g.fromSyscall(u)])),s=e.args.map(o=>g.fromSyscall(o));return new v({hash:t,function:n,env:r,args:s})}async run(){let e=await R.run(U.toSyscall(this));return g.fromSyscall(e)}};var fe=(a,e)=>({callSites:e.map(n=>({typeName:n.getTypeName(),functionName:n.getFunctionName(),methodName:n.getMethodName(),fileName:n.getFileName(),lineNumber:n.getLineNumber(),columnNumber:n.getColumnNumber(),isEval:n.isEval(),isNative:n.isNative(),isConstructor:n.isConstructor(),isAsync:n.isAsync(),isPromiseAll:n.isPromiseAll(),promiseIndex:n.getPromiseIndex()}))});var pe=async a=>{let e=N(1);return i.fromSyscall(await Y(e,a))};var W=(...a)=>{let e=a.map(t=>de(t)).join(" ");te(e)},de=a=>K(a,new WeakSet),K=(a,e)=>{switch(typeof a){case"string":return`"${a}"`;case"number":return a.toString();case"boolean":return a?"true":"false";case"undefined":return"undefined";case"object":return a===null?"null":ge(a,e);case"function":return`[function ${a.name??"(anonymous)"}]`;case"symbol":return"[symbol]";case"bigint":return a.toString()}},ge=(a,e)=>{if(e.has(a))return"[circular]";if(e.add(a),a instanceof Array)return`[${a.map(t=>K(t,e)).join(", ")}]`;if(a instanceof Error)return a.stack??"";if(a instanceof Promise)return"[promise]";{let t="";a.constructor!==void 0&&a.constructor.name!=="Object"&&(t=`${a.constructor.name} `);let n=Object.entries(a).map(([r,s])=>`${r}: ${K(s,e)}`);return`${t}{ ${n.join(", ")} }`}};var he=a=>{if(typeof a=="string")return a;{let{arch:e,os:t}=a;return`${e}_${t}`}},L;(n=>(n.is=r=>r==="amd64_linux"||r==="arm64_linux"||r==="amd64_macos"||r==="arm64_macos",n.arch=r=>{switch(r){case"amd64_linux":case"amd64_macos":return"amd64";case"arm64_linux":case"arm64_macos":return"arm64";default:throw new Error("Invalid system.")}},n.os=r=>{switch(r){case"amd64_linux":case"arm64_linux":return"linux";case"amd64_macos":case"arm64_macos":return"macos";default:throw new Error("Invalid system.")}}))(L||={});Object.defineProperties(Error,{prepareStackTrace:{value:fe}});var Ae={log:W};Object.defineProperties(globalThis,{console:{value:Ae}});var be={Artifact:i,Blob:f,Directory:c,File:y,Function:S,Path:m,Placeholder:d,Symlink:p,System:L,Template:l,Value:g,base64:q,blob:F,call:$,directory:ie,download:me,env:I,file:D,function:ce,hex:X,include:pe,json:ee,log:W,output:ye,path:h,placeholder:M,process:ue,resolve:A,symlink:le,system:he,template:O,toml:se,utf8:ne,yaml:oe};Object.defineProperties(globalThis,{tg:{value:be},t:{value:H}});})();
//# sourceMappingURL=global.js.map
