/* 00443856 FUN_00443856 */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */

undefined4 __thiscall
FUN_00443856(void *this,void *param_1,int param_2,void *param_3,undefined4 param_4,int param_5,
            int param_6)

{
  _ptiddata p_Var1;
  undefined4 local_30 [2];
  undefined4 local_28;
  undefined4 local_24;
  void *local_20;
  undefined4 uStack_c;
  undefined *local_8;
  
  local_8 = &DAT_0044e028;
  uStack_c = 0x443862;
  local_24 = 0;
  local_28 = *(undefined4 *)(param_2 + -4);
  local_20 = this;
  FUN_0043f086(local_30,*(undefined4 *)((int)param_1 + 0x18));
  __getptd();
  __getptd();
  p_Var1 = __getptd();
  p_Var1->_terminate = param_1;
  p_Var1 = __getptd();
  p_Var1->_unexpected = param_3;
  local_8 = (undefined *)0x1;
  local_20 = (void *)FUN_0043f11b(param_2,param_4,this,param_5,param_6);
  local_8 = (undefined *)0xffffffff;
  FUN_00443999();
  return local_20;
}
