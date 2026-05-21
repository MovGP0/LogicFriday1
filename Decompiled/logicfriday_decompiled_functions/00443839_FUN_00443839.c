/* 00443839 FUN_00443839 */

int __fastcall FUN_00443839(int *param_1)

{
  int iVar1;
  int in_EAX;
  int iVar2;
  
  iVar1 = param_1[1];
  iVar2 = *param_1 + in_EAX;
  if (-1 < iVar1) {
    iVar2 = iVar2 + *(int *)(*(int *)(iVar1 + in_EAX) + param_1[2]) + iVar1;
  }
  return iVar2;
}
