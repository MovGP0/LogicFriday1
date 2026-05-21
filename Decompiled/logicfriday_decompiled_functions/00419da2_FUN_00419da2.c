/* 00419da2 FUN_00419da2 */

undefined4 __thiscall FUN_00419da2(void *this,int param_1)

{
  int iVar1;
  uint uVar2;
  int iVar3;
  
  if (*(int *)(param_1 + 8) == -0x96) {
    iVar1 = *(int *)(param_1 + 0x10);
    if (iVar1 == -1) {
      return 0;
    }
    if ((*(uint *)(param_1 + 0xc) & 1) != 0) {
      iVar3 = *(int *)(param_1 + 0x14);
      if ((*(int *)((int)this + 0x8c) != 0) || (*(int *)((int)this + 0x90) != 0)) {
        if (iVar3 == 0) {
          *(undefined **)(param_1 + 0x20) = &DAT_0044c9a0;
        }
        else if (iVar3 < *(int *)((int)this + 0x70)) {
          *(undefined **)(param_1 + 0x20) = &DAT_0044add0;
        }
        else if ((iVar3 != *(int *)((int)this + 0x70)) && (iVar3 < *(int *)((int)this + 0x6c))) {
          if (*(int *)((int)this + 0x8c) == 0) {
            *(undefined **)(param_1 + 0x20) = &DAT_0044bbb4;
          }
          else {
            *(undefined **)(param_1 + 0x20) = &DAT_0044bbb0;
          }
        }
        return 0;
      }
      uVar2 = 1 << (((char)*(undefined4 *)((int)this + 0x70) - (char)iVar3) - 1U & 0x1f);
      if (*(int *)((int)this + 0x84) == 0) {
        if (iVar3 == 0) {
          if ((*(uint *)(*(int *)((int)this + 0x7c) + iVar1 * 0x48) & uVar2) == 0) {
            *(undefined **)(param_1 + 0x20) = &DAT_0044c998;
          }
          else {
            *(undefined **)(param_1 + 0x20) = &DAT_0044c99c;
          }
        }
        else if (iVar3 < *(int *)((int)this + 0x70)) {
          if ((*(uint *)(*(int *)((int)this + 0x7c) + iVar1 * 0x48) & uVar2) == 0) {
            *(undefined **)(param_1 + 0x20) = &DAT_0044bbb0;
          }
          else {
            *(undefined **)(param_1 + 0x20) = &DAT_0044bbb4;
          }
        }
        else {
          if ((iVar3 == *(int *)((int)this + 0x70)) || (*(int *)((int)this + 0x6c) + -1 < iVar3)) {
            return 0;
          }
          iVar3 = (iVar3 - *(int *)((int)this + 0x70)) + -1;
          if (*(int *)(iVar1 * 0x48 + *(int *)((int)this + 0x7c) + 8 + iVar3 * 4) == 1) {
            *(undefined **)(param_1 + 0x20) = &DAT_0044bbb4;
          }
          else if (*(int *)(iVar1 * 0x48 + *(int *)((int)this + 0x7c) + 8 + iVar3 * 4) == 2) {
            *(undefined **)(param_1 + 0x20) = &DAT_0044add0;
          }
          else if (*(int *)((int)this + 0x88) == 0) {
            *(undefined1 **)(param_1 + 0x20) = &DAT_0044ad26;
          }
          else {
            *(undefined **)(param_1 + 0x20) = &DAT_0044bbb0;
          }
        }
      }
      else if (iVar3 == 0) {
        if ((*(uint *)(*(int *)((int)this + 0x7c) + 4 + iVar1 * 0x48) & uVar2) == 0) {
          if ((*(uint *)(*(int *)((int)this + 0x7c) + iVar1 * 0x48) & uVar2) == 0) {
            *(undefined **)(param_1 + 0x20) = &DAT_0044c998;
          }
          else {
            *(undefined **)(param_1 + 0x20) = &DAT_0044c99c;
          }
        }
        else {
          *(undefined **)(param_1 + 0x20) = &DAT_0044c9a0;
        }
      }
      else if (iVar3 < *(int *)((int)this + 0x70)) {
        if ((*(uint *)(*(int *)((int)this + 0x7c) + 4 + iVar1 * 0x48) & uVar2) == 0) {
          if ((*(uint *)(*(int *)((int)this + 0x7c) + iVar1 * 0x48) & uVar2) == 0) {
            *(undefined **)(param_1 + 0x20) = &DAT_0044bbb0;
          }
          else {
            *(undefined **)(param_1 + 0x20) = &DAT_0044bbb4;
          }
        }
        else {
          *(undefined **)(param_1 + 0x20) = &DAT_0044add0;
        }
      }
      else {
        if ((iVar3 == *(int *)((int)this + 0x70)) || (*(int *)((int)this + 0x6c) + -1 < iVar3)) {
          return 0;
        }
        iVar3 = (iVar3 - *(int *)((int)this + 0x70)) + -1;
        if (*(int *)(iVar1 * 0x48 + *(int *)((int)this + 0x7c) + 8 + iVar3 * 4) == 1) {
          *(undefined **)(param_1 + 0x20) = &DAT_0044bbb4;
        }
        else if (*(int *)(iVar1 * 0x48 + *(int *)((int)this + 0x7c) + 8 + iVar3 * 4) == 2) {
          *(undefined **)(param_1 + 0x20) = &DAT_0044add0;
        }
        else {
          *(undefined1 **)(param_1 + 0x20) = &DAT_0044ad26;
        }
      }
    }
  }
  else if ((*(int *)(param_1 + 8) == -0xc) && (*(int *)((int)this + 0x80) != 0)) {
    if (*(int *)(param_1 + 0xc) == 1) {
      return 0x20;
    }
    if (*(int *)(param_1 + 0xc) == 0x10001) {
      iVar1 = *(int *)(param_1 + 0x24);
      if (((*(int *)(*(int *)((int)this + 0x7c) + 8 + iVar1 * 0x48) != 1) ||
          (*(int *)(*(int *)((int)this + 0x7c) + 0xc + iVar1 * 0x48) != 0)) &&
         ((*(int *)(*(int *)((int)this + 0x7c) + 8 + iVar1 * 0x48) != 0 ||
          (*(int *)(*(int *)((int)this + 0x7c) + 0xc + iVar1 * 0x48) != 1)))) {
        return 0;
      }
      *(undefined4 *)(param_1 + 0x34) = 0x9f9fff;
      return 2;
    }
  }
  return 0;
}
