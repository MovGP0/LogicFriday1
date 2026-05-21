/* 00444b32 FUN_00444b32 */

bool FUN_00444b32(void)

{
  bool bVar1;
  uint uVar2;
  uint uVar3;
  int iVar4;
  int *unaff_ESI;
  int iVar5;
  uint uVar6;
  
  if (DAT_004521b4 == 0) {
    return false;
  }
  iVar5 = unaff_ESI[5];
  if ((iVar5 != DAT_00452248) || (iVar5 != DAT_00452254)) {
    if (DAT_0046c7d4 == 0) {
      cvtdate(1,1,iVar5,1,0,0,0,0,0);
      cvtdate(0,1,unaff_ESI[5],5,0,0,0,0,0);
    }
    else {
      if (DAT_0046c7c0 != 0) {
        uVar6 = (uint)DAT_0046c7c6;
        uVar2 = 0;
        uVar3 = 0;
      }
      else {
        uVar2 = (uint)DAT_0046c7c4;
        uVar6 = 0;
        uVar3 = (uint)DAT_0046c7c6;
      }
      cvtdate(1,(uint)(DAT_0046c7c0 == 0),iVar5,uVar3,uVar2,uVar6,(uint)DAT_0046c7ca,
              (uint)DAT_0046c7cc,(uint)DAT_0046c7ce);
      if (DAT_0046c76c != 0) {
        uVar6 = (uint)DAT_0046c772;
        uVar2 = 0;
        uVar3 = 0;
        iVar5 = unaff_ESI[5];
      }
      else {
        uVar2 = (uint)DAT_0046c770;
        uVar6 = 0;
        uVar3 = (uint)DAT_0046c772;
        iVar5 = unaff_ESI[5];
      }
      cvtdate(0,(uint)(DAT_0046c76c == 0),iVar5,uVar3,uVar2,uVar6,(uint)DAT_0046c776,
              (uint)DAT_0046c778,(uint)DAT_0046c77a);
    }
  }
  iVar5 = unaff_ESI[7];
  if (DAT_0045224c < DAT_00452258) {
    if ((iVar5 < DAT_0045224c) || (DAT_00452258 < iVar5)) {
      return false;
    }
    if ((DAT_0045224c < iVar5) && (iVar5 < DAT_00452258)) {
      return true;
    }
  }
  else {
    if (iVar5 < DAT_00452258) {
      return true;
    }
    if (DAT_0045224c < iVar5) {
      return true;
    }
    if ((DAT_00452258 < iVar5) && (iVar5 < DAT_0045224c)) {
      return false;
    }
  }
  iVar4 = ((unaff_ESI[2] * 0x3c + unaff_ESI[1]) * 0x3c + *unaff_ESI) * 1000;
  if (iVar5 == DAT_0045224c) {
    bVar1 = DAT_00452250 <= iVar4;
  }
  else {
    bVar1 = iVar4 < DAT_0045225c;
  }
  return bVar1;
}
