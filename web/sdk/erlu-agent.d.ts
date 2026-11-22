export interface Action {name:string;description:string;input_schema:Record<string,unknown>;read_only:boolean}
export interface Response<T=unknown> {revision:number;dirty:boolean;result:T}
export interface State {revision:number;dirty:boolean;view:unknown}
export interface Agent {call<T=unknown>(action:string,args?:Record<string,unknown>,expected_revision?:number):Promise<Response<T>>;actions():Promise<{actions:Action[]}>;subscribe(listener:(state:State)=>void):()=>void;close():void}
export declare const PROTOCOL:'erlu.ai.v1';
export declare function connect(targetWindow:Window,targetOrigin:string,options?:{timeout?:number}):Promise<Agent>;
