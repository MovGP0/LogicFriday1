/* 0041e0c1 FUN_0041e0c1 */

int __thiscall FUN_0041e0c1(void *this,uint *param_1,undefined4 *param_2,undefined4 *param_3)

{
  int iVar1;
  
  if (((param_1 == (uint *)0x0) || (iVar1 = FUN_00421b02(this,param_1), iVar1 == 0)) &&
     (iVar1 = FUN_0042093b(this,1,"Entered by truthtable:"), iVar1 == 0)) {
    *(undefined4 *)((int)this + 0x23c) = 0;
    *param_2 = this;
    *param_3 = *(undefined4 *)((int)this + 0x268);
    iVar1 = 0;
  }
  return iVar1;
}
