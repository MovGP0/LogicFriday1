/* 0043ef9b FUN_0043ef9b */

undefined4 __cdecl
FUN_0043ef9b(EHExceptionRecord *param_1,EHRegistrationNode *param_2,EHRegistrationNode *param_3)

{
  undefined4 uVar1;
  
  if ((*(uint *)(param_1 + 4) & 0x66) != 0) {
    *(undefined4 *)(param_2 + 0x24) = 1;
    return 1;
  }
  ___InternalCxxFrameHandler
            (param_1,*(int *)(param_2 + 0xc),param_3,(_CONTEXT *)0x0,*(_s_FuncInfo **)(param_2 + 8),
             *(_s_FuncInfo **)(param_2 + 0x10),*(_s_HandlerType **)(param_2 + 0x14),'\x01');
  if (*(int *)(param_2 + 0x24) == 0) {
    _UnwindNestedFrames(param_2,param_1);
  }
                    /* WARNING: Could not recover jumptable at 0x0043f003. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  uVar1 = (**(code **)(param_2 + 0x18))();
  return uVar1;
}
