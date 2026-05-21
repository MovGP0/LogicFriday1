/* 00445956 FUN_00445956 */

byte * FUN_00445956(void)

{
  byte bVar1;
  int iVar2;
  byte *pbVar3;
  
  if (DAT_0046cd4c == 0) {
    ___initmbctable();
  }
  if (DAT_0046dd84 == (byte *)0x0) {
    pbVar3 = &DAT_0044ad26;
  }
  else {
    bVar1 = *DAT_0046dd84;
    pbVar3 = DAT_0046dd84;
    if (bVar1 != 0x22) {
      do {
        if (bVar1 < 0x21) goto LAB_004459b5;
        bVar1 = pbVar3[1];
        pbVar3 = pbVar3 + 1;
      } while( true );
    }
    pbVar3 = DAT_0046dd84 + 1;
    bVar1 = *pbVar3;
    if (bVar1 != 0x22) {
      do {
        if (bVar1 == 0) break;
        iVar2 = FUN_004484d9(bVar1);
        if (iVar2 != 0) {
          pbVar3 = pbVar3 + 1;
        }
        pbVar3 = pbVar3 + 1;
        bVar1 = *pbVar3;
      } while (bVar1 != 0x22);
      if (*pbVar3 != 0x22) goto LAB_004459b5;
    }
    do {
      pbVar3 = pbVar3 + 1;
LAB_004459b5:
    } while ((*pbVar3 != 0) && (*pbVar3 < 0x21));
  }
  return pbVar3;
}
