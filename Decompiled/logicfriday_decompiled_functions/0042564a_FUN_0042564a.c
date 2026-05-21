/* 0042564a FUN_0042564a */

void __thiscall FUN_0042564a(void *this,int param_1,int param_2,int param_3,int param_4,int param_5)

{
  int iVar1;
  
  if ((*(int *)(param_4 * 0xfc + *(int *)((int)this + 0x3a4)) == 0) ||
     (*(int *)(param_4 * 0xfc + *(int *)((int)this + 0x3a4)) == 9)) {
    *(undefined4 *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 4 + param_2 * 0x48) =
         *(undefined4 *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 4 + param_3 * 0x48);
  }
  else if (*(int *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 4 + param_3 * 0x48) <
           0x10) {
    if (*(int *)(param_4 * 0xfc + *(int *)((int)this + 0x3a4)) == 5) {
      iVar1 = *(int *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 4 + param_3 * 0x48);
      if (param_5 == 0) {
        if (iVar1 == 0) {
          *(undefined4 *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 4 + param_2 * 0x48)
               = 0xf;
        }
        else if (iVar1 == 0xf) {
          *(undefined4 *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 4 + param_2 * 0x48)
               = 0x28;
        }
      }
      else if (param_5 == 1) {
        *(int *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 4 + param_2 * 0x48) = iVar1;
      }
      else if (param_5 == 2) {
        if (iVar1 == 0) {
          *(undefined4 *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 4 + param_2 * 0x48)
               = 0x19;
        }
        else if (iVar1 == 0x28) {
          *(undefined4 *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 4 + param_2 * 0x48)
               = 0x19;
        }
      }
    }
    else if (*(int *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 4 + param_3 * 0x48) ==
             0) {
      *(undefined4 *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 4 + param_2 * 0x48) =
           0xf;
      if (param_5 != 0) {
        FUN_004258e1(this,param_4,param_5,0);
      }
    }
    else {
      *(undefined4 *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 4 + param_2 * 0x48) = 0
      ;
      if (*(int *)(*(int *)((int)this + 0x3a4) + 0x18 + param_4 * 0xfc) == 2) {
        if (param_5 != 1) {
          FUN_004258e1(this,param_4,param_5,1);
        }
      }
      else if (*(int *)(*(int *)((int)this + 0x3a4) + 0x18 + param_4 * 0xfc) == 3) {
        if (param_5 != 2) {
          FUN_004258e1(this,param_4,param_5,2);
        }
      }
      else if ((*(int *)(*(int *)((int)this + 0x3a4) + 0x18 + param_4 * 0xfc) == 4) &&
              (param_5 != 3)) {
        FUN_004258e1(this,param_4,param_5,3);
      }
    }
  }
  return;
}
