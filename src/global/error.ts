export function prepareStackTrace(
	_error: unknown,
	structuredStackTrace: Array<CallSite>,
) {
	let callSites = structuredStackTrace.map((callSite) => {
		return {
			typeName: callSite.getTypeName(),
			functionName: callSite.getFunctionName(),
			methodName: callSite.getMethodName(),
			fileName: callSite.getFileName(),
			lineNumber: callSite.getLineNumber(),
			columnNumber: callSite.getColumnNumber(),
			isEval: callSite.isEval(),
			isNative: callSite.isNative(),
			isConstructor: callSite.isConstructor(),
			isAsync: callSite.isAsync(),
			isPromiseAll: callSite.isPromiseAll(),
			// isPromiseAny: callSite.isPromiseAny(),
			promiseIndex: callSite.getPromiseIndex(),
		};
	});
	return { callSites };
}

/** This type is derived from <https://v8.dev/docs/stack-trace-api#customizing-stack-traces>. */
type CallSite = {
	/** getTypeName: returns the type of this as a string. This is the name of the function stored in the constructor field of this, if available, otherwise the object’s [[Class]] internal property. **/
	getTypeName(): string;

	/** getFunctionName: returns the name of the current function, typically its name property. If a name property is not available an attempt is made to infer a name from the function’s context. **/
	getFunctionName(): string;

	/** getMethodName: returns the name of the property of this or one of its prototypes that holds the current function **/
	getMethodName(): string;

	/** getFileName: if this function was defined in a script returns the name of the script **/
	getFileName(): string | undefined;

	/** getLineNumber: if this function was defined in a script returns the current line number **/
	getLineNumber(): number | undefined;

	/** getColumnNumber: if this function was defined in a script returns the current column number **/
	getColumnNumber(): number | undefined;

	/** getEvalOrigin: if this function was created using a call to eval returns a string representing the location where eval was called **/
	getEvalOrigin(): any | undefined;

	/** isEval: does this call take place in code defined by a call to eval? **/
	isEval(): boolean;

	/** isNative: is this call in native V8 code? **/
	isNative(): boolean;

	/** isConstructor: is this a constructor call? **/
	isConstructor(): boolean;

	/** isAsync: is this an async call (i.e. await, Promise.all(), or Promise.any())? **/
	isAsync(): boolean;

	/** isPromiseAll: is this an async call to Promise.all()? **/
	isPromiseAll(): boolean;

	// /** isPromiseAny: is this an async call to Promise.any()? **/
	// isPromiseAny(): boolean;

	/** getPromiseIndex: returns the index of the promise element that was followed in Promise.all() or Promise.any() for async stack traces, or null if the CallSite is not an async Promise.all() or Promise.any() call. **/
	getPromiseIndex(): number | null;
};
