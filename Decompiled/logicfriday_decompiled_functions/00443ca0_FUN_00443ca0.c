/* 00443ca0 FUN_00443ca0 */

void __cdecl
FUN_00443ca0(EHExceptionRecord *param_1,int param_2,EHRegistrationNode *param_3,_CONTEXT *param_4,
            _s_FuncInfo *param_5,char param_6,_s_FuncInfo *param_7,_s_HandlerType *param_8)

{
  bool bVar1;
  _ptiddata p_Var2;
  undefined3 extraout_var;
  _s_TryBlockMapEntry *p_Var3;
  int iVar4;
  EHRegistrationNode *unaff_EBX;
  _s_ThrowInfo *unaff_ESI;
  _s_TryBlockMapEntry *unaff_EDI;
  uchar uVar5;
  uint local_24;
  _s_CatchableType *local_20;
  int local_1c;
  int local_18;
  int local_14;
  int *local_10;
  uint local_c;
  _s_TryBlockMapEntry *local_8;
  
  local_1c = *(int *)(param_2 + 8);
  local_20 = (_s_CatchableType *)((uint)local_20 & 0xffffff00);
  if ((local_1c < -1) || (param_5->maxState <= local_1c)) {
    _inconsistency();
  }
  if (*(int *)param_1 == -0x1f928c9d) {
    if (((*(int *)(param_1 + 0x10) == 3) && (*(int *)(param_1 + 0x14) == 0x19930520)) &&
       (*(int *)(param_1 + 0x1c) == 0)) {
      p_Var2 = __getptd();
      if (p_Var2->_terminate == (void *)0x0) {
        return;
      }
      p_Var2 = __getptd();
      param_1 = p_Var2->_terminate;
      p_Var2 = __getptd();
      param_3 = p_Var2->_unexpected;
      local_20 = (_s_CatchableType *)CONCAT31(local_20._1_3_,1);
      bVar1 = FID_conflict__ValidateRead(param_1,1);
      if (CONCAT31(extraout_var,bVar1) == 0) {
        _inconsistency();
      }
      if (*(int *)param_1 != -0x1f928c9d) goto LAB_00443e57;
      if (((*(int *)(param_1 + 0x10) == 3) && (*(int *)(param_1 + 0x14) == 0x19930520)) &&
         (*(int *)(param_1 + 0x1c) == 0)) {
        _inconsistency();
      }
    }
    iVar4 = local_1c;
    if (((*(int *)param_1 == -0x1f928c9d) && (*(int *)(param_1 + 0x10) == 3)) &&
       (*(int *)(param_1 + 0x14) == 0x19930520)) {
      p_Var3 = _GetRangeOfTrysToCheck(param_5,(int)param_7,local_1c,&local_c,&local_24);
      local_8 = p_Var3;
      if (local_c < local_24) {
        do {
          if ((p_Var3->tryLow <= iVar4) && (iVar4 <= p_Var3->tryHigh)) {
            local_8 = p_Var3;
            for (local_18 = p_Var3->nCatches; 0 < local_18; local_18 = local_18 + -1) {
              local_10 = *(int **)(*(int *)(param_1 + 0x1c) + 0xc);
              for (local_14 = *local_10; local_10 = local_10 + 1, 0 < local_14;
                  local_14 = local_14 + -1) {
                uVar5 = (uchar)*local_10;
                iVar4 = TypeMatch(*(_s_HandlerType **)(param_1 + 0x1c),(_s_CatchableType *)unaff_EDI
                                  ,unaff_ESI);
                p_Var3 = local_8;
                if (iVar4 != 0) {
                  CatchIt(param_1,param_3,param_4,param_5,param_7,param_8,local_20,unaff_EDI,
                          (int)unaff_ESI,unaff_EBX,uVar5);
                  goto LAB_00443e2a;
                }
              }
            }
          }
LAB_00443e2a:
          local_c = local_c + 1;
          p_Var3 = p_Var3 + 1;
          iVar4 = local_1c;
          local_8 = p_Var3;
        } while (local_c < local_24);
      }
      if (param_6 == '\0') {
        return;
      }
      ___DestructExceptionObject((int)param_1);
      return;
    }
  }
LAB_00443e57:
  if (param_6 == '\0') {
    FUN_00443be0(param_1,param_2,param_3,param_4,param_5,local_1c,param_7,param_8);
    return;
  }
  terminate();
  return;
}
