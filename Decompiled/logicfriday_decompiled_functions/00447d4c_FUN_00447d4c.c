/* 00447d4c FUN_00447d4c */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

int * __cdecl FUN_00447d4c(undefined4 param_1,undefined4 param_2,int *param_3,uint *param_4)

{
  int *piVar1;
  uint *puVar2;
  int iVar3;
  uint unaff_retaddr;
  undefined4 in_stack_ffffffb8;
  undefined2 uVar4;
  short local_30;
  char local_2e;
  uint local_2c [6];
  uint local_14;
  uint uStack_10;
  undefined2 uStack_c;
  uint local_8;
  
  uVar4 = (undefined2)((uint)in_stack_ffffffb8 >> 0x10);
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  ___dtold(&local_14,&param_1);
  iVar3 = FUN_00448c1d(local_14,uStack_10,(short *)CONCAT22(uVar4,uStack_c),0x11,0,&local_30);
  puVar2 = param_4;
  piVar1 = param_3;
  param_3[2] = iVar3;
  *param_3 = (int)local_2e;
  param_3[1] = (int)local_30;
  FUN_0043ebd0(param_4,local_2c);
  piVar1[3] = (int)puVar2;
  return piVar1;
}
