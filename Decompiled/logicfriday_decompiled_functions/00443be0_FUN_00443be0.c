/* 00443be0 FUN_00443be0 */

void __cdecl
FUN_00443be0(EHExceptionRecord *param_1,undefined4 param_2,EHRegistrationNode *param_3,
            _CONTEXT *param_4,_s_FuncInfo *param_5,int param_6,_s_FuncInfo *param_7,
            _s_HandlerType *param_8)

{
  TypeDescriptor *pTVar1;
  _ptiddata p_Var2;
  int iVar3;
  _s_TryBlockMapEntry *p_Var4;
  _s_TryBlockMapEntry *unaff_EBX;
  EHRegistrationNode *unaff_ESI;
  int unaff_EDI;
  uint extraout_var;
  uint uVar5;
  uint local_8;
  
  if ((*(int *)param_1 != -0x7ffffffd) &&
     (((p_Var2 = __getptd(), uVar5 = extraout_var, p_Var2->_NLG_dwCode == 0 ||
       (iVar3 = FUN_0043f16c((undefined4 *)param_1,param_2,param_3,param_4,param_5,param_7,param_8),
       iVar3 == 0)) &&
      (p_Var4 = _GetRangeOfTrysToCheck
                          (param_5,(int)param_7,param_6,&local_8,(uint *)&stack0xfffffff4),
      local_8 < uVar5)))) {
    do {
      if (((p_Var4->tryLow <= param_6) && (param_6 <= p_Var4->tryHigh)) &&
         ((pTVar1 = p_Var4->pHandlerArray[p_Var4->nCatches + -1].pType,
          pTVar1 == (TypeDescriptor *)0x0 || ((char)pTVar1[1].hash == '\0')))) {
        CatchIt(param_1,param_3,param_4,param_5,param_7,param_8,(_s_CatchableType *)0x1,unaff_EBX,
                unaff_EDI,unaff_ESI,(uchar)uVar5);
      }
      local_8 = local_8 + 1;
      p_Var4 = p_Var4 + 1;
    } while (local_8 < uVar5);
  }
  return;
}
