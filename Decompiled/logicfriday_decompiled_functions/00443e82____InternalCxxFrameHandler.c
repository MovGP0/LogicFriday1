/* 00443e82 ___InternalCxxFrameHandler */

/* Library Function - Single Match
    ___InternalCxxFrameHandler
   
   Library: Visual Studio 2003 Release */

undefined4 __cdecl
___InternalCxxFrameHandler
          (EHExceptionRecord *param_1,int param_2,EHRegistrationNode *param_3,_CONTEXT *param_4,
          _s_FuncInfo *param_5,_s_FuncInfo *param_6,_s_HandlerType *param_7,char param_8)

{
  undefined4 uVar1;
  
  if ((param_5->magicNumber_and_bbtFlags & 0x1fffffff) != 0x19930520) {
    _inconsistency();
  }
  if (((byte)param_1[4] & 0x66) == 0) {
    if (param_5->nTryBlocks != 0) {
      if (((*(int *)param_1 == -0x1f928c9d) && (0x19930520 < *(uint *)(param_1 + 0x14))) &&
         (*(code **)(*(int *)(param_1 + 0x1c) + 8) != (code *)0x0)) {
        uVar1 = (**(code **)(*(int *)(param_1 + 0x1c) + 8))
                          (param_1,param_2,param_3,param_4,param_5,param_6,param_7,param_8);
        return uVar1;
      }
      FUN_00443ca0(param_1,param_2,param_3,param_4,param_5,param_8,param_6,param_7);
    }
  }
  else if ((param_5->maxState != 0) && (param_6 == (_s_FuncInfo *)0x0)) {
    ___FrameUnwindToState(param_2,param_4,(int)param_5,-1);
  }
  return 1;
}
